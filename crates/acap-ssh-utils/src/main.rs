use std::path::PathBuf;

use acap_ssh_utils::{run_as_package, sync_package};
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
    PatchAndRun(PatchAndRun),
}

#[derive(Clone, Debug, Parser)]
struct PatchAndRun {
    /// Name of app to patch and run as.
    package: String,
    /// Paths to upload before uploading
    #[arg(value_parser = parse_path_pair)]
    paths: Vec<(String, Option<String>)>,
    /// Environment variables to override on the remote host.
    ///
    /// Can be specified multiple times.
    #[clap(short, long)]
    #[arg(value_parser = parse_env_pair)]
    environment: Vec<(String, String)>,
}

fn parse_path_pair(s: &str) -> anyhow::Result<(String, Option<String>)> {
    if let Some((dst, src)) = s.split_once(':') {
        Ok((dst.to_string(), Some(src.to_string())))
    } else {
        Ok((s.to_string(), None))
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
    let Command::PatchAndRun(PatchAndRun {
        paths,
        package,
        environment,
    }) = cli.command;

    sync_package(
        &std::env::current_dir().unwrap_or_default(),
        paths
            .into_iter()
            .map(|(k, v)| (PathBuf::from(k), v.map(PathBuf::from)))
            .collect(),
        &cli.username,
        &cli.password,
        &cli.host,
        &package,
    )?;
    run_as_package(
        &cli.username,
        &cli.password,
        &cli.host,
        &package,
        environment
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect(),
        &[],
    )
}
