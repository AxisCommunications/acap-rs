use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use url::Host;

use acap_ssh_utils::run;

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
    /// Hostname or IP address of the device.
    #[arg(value_parser = url::Host::parse)]
    host: Host,
    /// The username to use for the ssh connection.
    #[clap(short, long, default_value = "root")]
    username: String,
    /// The password to use for the ssh connection.
    #[clap(short, long)]
    password: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    /// Patch app on device and run it attached to the terminal.
    Run(Run),
}

#[derive(Clone, Debug, Parser)]
struct Run {
    /// Path to the executable to upload.
    executable: PathBuf,
    /// Name of app to patch and run as.
    package: String,
    /// Environment variables to override on the remote host.
    ///
    /// Can be specified multiple times.
    #[clap(short, long)]
    #[arg(value_parser = parse_key_value_pair)]
    environment: Vec<(String, String)>,
}

fn parse_key_value_pair(s: &str) -> anyhow::Result<(String, String)> {
    s.split_once('=')
        .context("Delimiter '=' not found")
        .map(|(k, v)| (k.to_string(), v.to_string()))
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let Command::Run(cmd) = cli.command;
    run(
        &cli.username,
        &cli.password,
        &cli.host,
        &cmd.executable,
        Some(&cmd.package),
        cmd.environment
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect(),
    )
}
