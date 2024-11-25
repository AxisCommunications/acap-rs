#![forbid(unsafe_code)]
use std::{ffi::OsString, fs::File, str::FromStr};

use acap_vapix::{applications_control, basic_device_info, HttpClient};
use cargo_acap_build::Architecture;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use log::debug;
use url::Host;

use crate::commands::{
    build_command::BuildCommand, completions_command::CompletionsCommand,
    control_command::ControlCommand, install_command::InstallCommand, run_command::RunCommand,
    test_command::TestCommand,
};

mod commands;

/// Tools for developing ACAP apps using Rust
#[derive(Parser)]
#[clap(verbatim_doc_comment, version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn exec(self) -> anyhow::Result<()> {
        match self.command {
            Commands::Build(cmd) => cmd.exec()?,
            Commands::Install(cmd) => cmd.exec().await?,
            Commands::Run(cmd) => cmd.exec().await?,
            Commands::Test(cmd) => cmd.exec().await?,
            Commands::Completions(cmd) => cmd.exec(Cli::command())?,
            Commands::Start(cmd) => cmd.exec(applications_control::Action::Start).await?,
            Commands::Stop(cmd) => cmd.exec(applications_control::Action::Stop).await?,
            Commands::Restart(cmd) => cmd.exec(applications_control::Action::Restart).await?,
            // The Cargo command `remove` is not the inverse of `install`, instead it is the inverse
            // of `add`. Furthermore `install` maps to the verb _upload_ in VAPIX.
            // TODO: Consider renaming this and other commands for consistency.
            Commands::Remove(cmd) => cmd.exec(applications_control::Action::Remove).await?,
        }
        Ok(())
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Build app(s) with release profile.
    Build(BuildCommand),
    /// Build app(s) and run on the device.
    Run(RunCommand),
    /// Build app(s) in test mode and run on the device.
    Test(TestCommand),
    /// Build app(s) with release profile and install on the device.
    Install(InstallCommand),
    /// Start app on device.
    Start(ControlCommand),
    /// Stop app on device.
    Stop(ControlCommand),
    /// Restart app on device.
    Restart(ControlCommand),
    /// Remove app form device.
    Remove(ControlCommand),
    /// Print shell completion script for this program
    ///
    /// In `zsh` this can be used to enable completions in the current shell like
    /// `cargo-acap-sdk completions zsh | source /dev/stdin`.
    Completions(CompletionsCommand),
}

// TODO: Include package selection for better completions and help messages.
#[derive(clap::Args, Debug, Clone)]
struct BuildOptions {
    /// Pass additional arguments to `cargo build`.
    ///
    /// Beware that not all incompatible arguments have been documented.
    args: Vec<String>,
}

impl BuildOptions {
    async fn resolve(self, deploy_options: &DeployOptions) -> anyhow::Result<ResolvedBuildOptions> {
        let Self { args } = self;
        // TODO: Consider using `get_properties` instead.
        let target = basic_device_info::Client::new(&deploy_options.http_client().await?)
            .get_all_properties()
            .send()
            .await?
            .property_list
            .restricted
            .architecture
            .parse()?;
        Ok(ResolvedBuildOptions { target, args })
    }
}

#[derive(clap::Args, Debug, Clone)]
pub struct ResolvedBuildOptions {
    /// Architecture of the device to build for.
    #[arg(long, env = "AXIS_DEVICE_ARCH")]
    target: ArchAbi,
    /// Pass additional arguments to `cargo build`.
    ///
    /// Beware that not all incompatible arguments have been documented.
    args: Vec<String>,
}

#[derive(clap::Args, Debug, Clone)]
struct DeployOptions {
    /// Hostname or IP address of the device.
    #[arg(long, value_parser = url::Host::parse, env="AXIS_DEVICE_IP")]
    host: Host,
    /// Username of SSH- and/or VAPIX-account to authenticate as.
    ///
    /// It is up to the user to ensure that these have been created on the device as needed.
    #[clap(long, env = "AXIS_DEVICE_USER", default_value = "root")]
    user: String,
    /// Password of SSH- and/or VAPIX-account to authenticate as.
    ///
    /// It is up to the user to ensure that these have been created on the device as needed.
    // TODO: Consider disallowing passing password as arguments.
    #[clap(long, env = "AXIS_DEVICE_PASS", default_value = "pass")]
    pass: String,
}

impl DeployOptions {
    pub async fn http_client(&self) -> anyhow::Result<HttpClient> {
        // This takes about 200ms on my setup. It's not terrible since successful requests to
        // applications control take on the order of seconds, but it is a bit annoying on failing
        // requests that take 200-500ms. But since `from_host` tries more secure configurations
        // first this will probably improve as https and digest support are added and
        // `device-manager` is changed to set up the devices accordingly.
        // TODO: Consider allowing the resolved settings to be cached or configured
        let Self { host, user, pass } = self;
        HttpClient::from_host(host)
            .await?
            .automatic_auth(user, pass)
            .await
    }
}

// TODO: Figure out what to call this.
// This is sometimes called just "architecture" but in other contexts arch refers to the first
// part: https://clang.llvm.org/docs/CrossCompilation.html#target-triple
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, ValueEnum)]
enum ArchAbi {
    Aarch64,
    Armv7hf,
}

impl FromStr for ArchAbi {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "aarch64" => Ok(Self::Aarch64),
            "armv7hf" => Ok(Self::Armv7hf),
            _ => Err(anyhow::anyhow!("Unrecognized variant {s}")),
        }
    }
}

impl From<ArchAbi> for Architecture {
    fn from(val: ArchAbi) -> Self {
        match val {
            ArchAbi::Aarch64 => Architecture::Aarch64,
            ArchAbi::Armv7hf => Architecture::Armv7hf,
        }
    }
}

fn normalized_args() -> Vec<OsString> {
    let mut args: Vec<_> = std::env::args_os().collect();
    if let Some(command) = args.get(1) {
        if command.to_string_lossy() == "acap-sdk" {
            args.remove(1);
        }
    }
    args
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_file = if std::env::var_os("RUST_LOG").is_none() {
        if let Some(runtime_dir) = dirs::runtime_dir() {
            let path = runtime_dir.join("cargo-acap-sdk.log");
            let target = env_logger::Target::Pipe(Box::new(File::create(&path)?));
            let mut builder = env_logger::Builder::from_env(env_logger::Env::default());
            builder.target(target).filter_level(log::LevelFilter::Debug);
            builder.init();
            Some(path)
        } else {
            None
        }
    } else {
        env_logger::init();
        None
    };
    debug!("Logging initialized");

    match Cli::parse_from(normalized_args()).exec().await {
        Ok(()) => Ok(()),
        Err(e) => {
            if let Some(log_file) = log_file {
                Err(e.context(format!("A detailed log has been saved to {log_file:?}")))
            } else {
                Err(e)
            }
        }
    }
}
