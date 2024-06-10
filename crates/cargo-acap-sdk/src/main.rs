use std::{
    ffi::OsString,
    fmt::{Display, Formatter},
    fs::File,
};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use log::debug;
use url::Host;

use crate::commands::{
    build_command::BuildCommand, completions_command::CompletionsCommand,
    containerize_command::ContainerizeCommand, install_command::InstallCommand,
    run_command::RunCommand, test_command::TestCommand,
};

mod cargo_utils;
mod command_utils;

mod acap_utils;
mod commands;
mod docker_utils;

/// ACAP analog to `cargo` for building and deploying apps.
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub fn exec(self) -> anyhow::Result<()> {
        match self.command {
            Commands::Build(cmd) => cmd.exec()?,
            Commands::Install(cmd) => cmd.exec()?,
            Commands::Run(cmd) => cmd.exec()?,
            Commands::Test(cmd) => cmd.exec()?,
            Commands::Containerize(cmd) => cmd.exec()?,
            Commands::Completions(cmd) => cmd.exec(Cli::command())?,
            _ => todo!(),
        }
        Ok(())
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Build app(s)
    Build(BuildCommand),
    /// Build executable for app(s) and run on the device, impersonating a user or the app.
    ///
    /// `--target` must be given exactly once and match the device.
    Run(RunCommand),
    /// Build test(s) and run on the device, impersonating a user or the app.
    ///
    /// `--target` must be given exactly once and match the device.
    Test(TestCommand),
    /// Build app(s) and install on the device.
    ///
    /// `--target` must be given exactly once and match the device.
    ///
    /// TODO: Implement without docker
    Install(InstallCommand),
    /// TODO: Implement;
    /// Start app(s) on the device
    Start,
    /// TODO: Implement;
    /// Stop app(s) on the device
    Stop,
    /// TODO: Implement;
    /// Uninstall app(s) on the device
    Uninstall,
    /// Run the provided program in a container
    Containerize(ContainerizeCommand),
    /// Print shell completion script for this program
    ///
    /// In `zsh` this can be used to enable completions in the current shell like
    /// `cargo-acap-sdk completions zsh | source /dev/stdin`.
    Completions(CompletionsCommand),
}

#[derive(clap::Args, Debug, Clone)]
struct BuildOptions {
    #[command(flatten)]
    verbosity: Verbosity,
    /// If given, build only matching packages
    #[arg(short, long)]
    package: Option<String>,
    /// If given, build only for the given architecture(s).
    /// Can be used multiple times.
    ///
    // TODO: Query the device for its architecture.
    //  Architecture of the device to test on.
    #[arg(long)]
    target: Vec<ArchAbi>,
}

impl BuildOptions {
    pub fn targets(&self) -> Vec<ArchAbi> {
        if self.target.is_empty() {
            vec![ArchAbi::Aarch64, ArchAbi::Armv7hf]
        } else {
            self.target.clone()
        }
    }
}

#[derive(clap::Args, Debug, Clone)]
struct DeployOptions {
    /// Hostname or IP address of the device.
    #[arg(long)]
    #[arg(value_parser = url::Host::parse)]
    address: Host,
    /// Username of SSH- and/or VAPIX-account to authenticate as.
    ///
    /// It is up to the user to ensure that these have been created on the device as needed.
    #[arg(long, default_value = "root")]
    username: String,
    /// Password of SSH- and/or VAPIX-account to authenticate as.
    ///
    /// It is up to the user to ensure that these have been created on the device as needed.
    // TODO: Consider not passing password as arguments.
    #[arg(long)]
    password: String,
}

#[derive(clap::Args, Debug, Clone, Default)]
pub struct Verbosity {
    /// Control Cargo verbosity
    #[arg(long, short, action = clap::ArgAction::SetTrue, conflicts_with = "verbose")]
    quiet: bool,

    /// Control Cargo verbosity
    #[arg(long, short, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl Verbosity {
    pub fn arg(&self) -> Option<String> {
        match (self.quiet, self.verbose.min(2)) {
            (false, 0) => None,
            (false, 1) => Some("-v".to_string()),
            (false, 2) => Some("-vv".to_string()),
            (true, _) => Some("-q".to_string()),
            (_, _) => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Target {
    Aarch64,
    Armv7hf,
}

impl From<ArchAbi> for Target {
    fn from(val: ArchAbi) -> Self {
        match val {
            ArchAbi::Aarch64 => Target::Aarch64,
            ArchAbi::Armv7hf => Target::Armv7hf,
        }
    }
}
impl Target {
    fn triple(&self) -> &'static str {
        match self {
            Target::Aarch64 => "aarch64-unknown-linux-gnu",
            Target::Armv7hf => "thumbv7neon-unknown-linux-gnueabihf",
        }
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

impl Display for ArchAbi {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ArchAbi::Aarch64 => write!(f, "aarch64"),
            ArchAbi::Armv7hf => write!(f, "armv7hf"),
        }
    }
}

impl From<Target> for ArchAbi {
    fn from(value: Target) -> Self {
        match value {
            Target::Aarch64 => Self::Aarch64,
            Target::Armv7hf => Self::Armv7hf,
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

fn main() -> anyhow::Result<()> {
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

    match Cli::parse_from(normalized_args()).exec() {
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
