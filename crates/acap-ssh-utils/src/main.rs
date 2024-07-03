use std::path::PathBuf;

use acap_ssh_utils::{patch_package, run_other, run_package};
use anyhow::Context;
use clap::{Parser, Subcommand};
use url::Host;

/// Utilities for interacting with Axis devices over SSH.
///
/// The commands assume that the user has already
/// - installed `scp`, `ssh` and `sshpass`,
/// - added the device to the `known_hosts` file,
/// - enabled SSH on the device,
/// - configured the SSH user with a password and the necessary permissions, and
/// - installed any apps that will be impersonated.
#[derive(Clone, Debug, Parser)]
#[clap(verbatim_doc_comment)]
struct Cli {
    #[command(flatten)]
    netloc: Netloc,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Debug, Parser)]
struct Netloc {
    /// Hostname or IP address of the device.
    #[arg(long, value_parser = url::Host::parse)]
    host: Host,
    /// The username to use for the ssh connection.
    #[clap(short, long, default_value = "root")]
    username: String,
    /// The password to use for the ssh connection.
    #[clap(short, long)]
    password: String,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    // TODO: Reconsider these names.
    /// Patch app on device and run it attached to the terminal.
    Patch(Patch),
    RunApp(RunApp),
    RunOther(RunOther),
}

#[derive(Clone, Debug, Parser)]
struct Patch {
    /// `.eap` file to upload.
    package: PathBuf,
}

impl Patch {
    fn exec(self, netloc: Netloc) -> anyhow::Result<()> {
        patch_package(
            &self.package,
            &netloc.username,
            &netloc.password,
            &netloc.host,
        )
    }
}

#[derive(Clone, Debug, Parser)]
struct RunApp {
    /// Name of package to run.
    package: String,
    /// Environment variables to override on the remote host.
    ///
    /// Can be specified multiple times.
    #[clap(short, long)]
    #[arg(value_parser = parse_env_pair)]
    environment: Vec<(String, String)>,
}

impl RunApp {
    fn exec(self, netloc: Netloc) -> anyhow::Result<()> {
        run_package(
            &netloc.username,
            &netloc.password,
            &netloc.host,
            &self.package,
            self.environment
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect(),
        )
    }
}

#[derive(Clone, Debug, Parser)]
struct RunOther {
    /// Location of executable to run.
    package: PathBuf,
    /// Environment variables to override on the remote host.
    ///
    /// Can be specified multiple times.
    #[clap(short, long)]
    #[arg(value_parser = parse_env_pair)]
    environment: Vec<(String, String)>,
}

impl RunOther {
    fn exec(self, netloc: Netloc) -> anyhow::Result<()> {
        run_other(
            &self.package,
            &netloc.username,
            &netloc.password,
            &netloc.host,
            self.environment
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect(),
        )
    }
}

fn parse_env_pair(s: &str) -> anyhow::Result<(String, String)> {
    s.split_once('=')
        .context("Delimiter '=' not found")
        .map(|(k, v)| (k.to_string(), v.to_string()))
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let netloc = cli.netloc;
    match cli.command {
        Command::Patch(cmd) => cmd.exec(netloc),
        Command::RunApp(cmd) => cmd.exec(netloc),
        Command::RunOther(cmd) => cmd.exec(netloc),
    }
}
