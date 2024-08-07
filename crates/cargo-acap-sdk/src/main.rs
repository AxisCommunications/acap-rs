use std::{ffi::OsString, fs::File};

use cargo_acap_build::Architecture;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use log::debug;
use url::Host;

use crate::commands::{
    build_command::BuildCommand, completions_command::CompletionsCommand,
    install_command::InstallCommand, run_command::RunCommand, test_command::TestCommand,
};

mod command_utils;

mod commands;

/// Tools for developing ACAP apps using Rust
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
            Commands::Completions(cmd) => cmd.exec(Cli::command())?,
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
    /// Print shell completion script for this program
    ///
    /// In `zsh` this can be used to enable completions in the current shell like
    /// `cargo-acap-sdk completions zsh | source /dev/stdin`.
    Completions(CompletionsCommand),
}

// TODO: Include package selection for better completions and help messages.
#[derive(clap::Args, Debug, Clone)]
struct BuildOptions {
    // TODO: Query the device for its architecture.
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
    #[clap(long, env = "AXIS_DEVICE_USER")]
    user: String,
    /// Password of SSH- and/or VAPIX-account to authenticate as.
    ///
    /// It is up to the user to ensure that these have been created on the device as needed.
    // TODO: Consider disallowing passing password as arguments.
    #[clap(long, env = "AXIS_DEVICE_PASS")]
    pass: String,
}

// TODO: Figure out what to call this.
// This is sometimes called just "architecture" but in other contexts arch refers to the first
// part: https://clang.llvm.org/docs/CrossCompilation.html#target-triple
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, ValueEnum)]
enum ArchAbi {
    Aarch64,
    Armv7hf,
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
