#![forbid(unsafe_code)]
use std::{env, fs::File, path::PathBuf};

use acap_ssh_utils::{patch_package, run_other, run_package};
use anyhow::Context;
use clap::{Parser, Subcommand};
use cli_version::version_with_commit_id;
use log::debug;
use url::Host;

/// Utilities for interacting with Axis devices over SSH.
///
/// The commands assume that the user has already
/// - installed `scp`, `ssh` and `sshpass`,
/// - added the device to the `known_hosts` file,
/// - enabled SSH on the device,
/// - configured the SSH user with a password and the necessary permissions, and
/// - installed any apps that will be impersonated.
///
/// # Warning
///
/// Neither the ability to patch an already installed app using SSH nor to run an installed app
/// with stdout attached to the terminal are officially supported use cases. As such all commands
/// provided by this program may stop working on future versions AXIS OS.
#[derive(Clone, Debug, Parser)]
#[clap(verbatim_doc_comment, version = version_with_commit_id!())]
struct Cli {
    #[command(flatten)]
    netloc: Netloc,
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    fn exec(self) -> anyhow::Result<()> {
        let Self { netloc, command } = self;
        match command {
            Command::Patch(cmd) => cmd.exec(netloc),
            Command::RunApp(cmd) => cmd.exec(netloc),
            Command::RunOther(cmd) => cmd.exec(netloc),
        }
    }
}

#[derive(Clone, Debug, Parser)]
struct Netloc {
    /// Hostname or IP address of the device.
    #[arg(long, value_parser = url::Host::parse, env="AXIS_DEVICE_IP")]
    host: Host,
    /// The username to use for the ssh connection.
    #[clap(short, long, env = "AXIS_DEVICE_USER", default_value = "root")]
    user: String,
    /// The password to use for the ssh connection.
    #[clap(short, long, env = "AXIS_DEVICE_PASS", default_value = "pass")]
    pass: String,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    /// Patch app on device.
    Patch(Patch),
    /// Run app on device, sending output to the terminal.
    RunApp(RunApp),
    /// Run any executable on device, sending output to the terminal.
    RunOther(RunOther),
}

#[derive(Clone, Debug, Parser)]
struct Patch {
    /// `.eap` file to upload.
    package: PathBuf,
}

impl Patch {
    fn exec(self, netloc: Netloc) -> anyhow::Result<()> {
        patch_package(&self.package, &netloc.user, &netloc.pass, &netloc.host)
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
    /// Pass additional arguments to the remote program.
    args: Vec<String>,
}

impl RunApp {
    fn exec(self, netloc: Netloc) -> anyhow::Result<()> {
        run_package(
            &netloc.user,
            &netloc.pass,
            &netloc.host,
            &self.package,
            self.environment
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect(),
            &self.args.iter().map(String::as_str).collect::<Vec<_>>(),
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
    /// Pass additional arguments to the remote program.
    args: Vec<String>,
}

impl RunOther {
    fn exec(self, netloc: Netloc) -> anyhow::Result<()> {
        run_other(
            &self.package,
            &netloc.user,
            &netloc.pass,
            &netloc.host,
            self.environment
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect(),
            &self.args.iter().map(String::as_str).collect::<Vec<_>>(),
        )
    }
}

fn parse_env_pair(s: &str) -> anyhow::Result<(String, String)> {
    s.split_once('=')
        .context("Delimiter '=' not found")
        .map(|(k, v)| (k.to_string(), v.to_string()))
}

fn main() -> anyhow::Result<()> {
    let log_file = if env::var_os("RUST_LOG").is_none() {
        let dir = dirs::runtime_dir().unwrap_or(env::temp_dir());
        let path = dir.join("cargo-acap-sdk.log");
        let target = env_logger::Target::Pipe(Box::new(File::create(&path)?));
        let mut builder = env_logger::Builder::from_env(env_logger::Env::default());
        builder.target(target).filter_level(log::LevelFilter::Debug);
        builder.init();
        Some(path)
    } else {
        env_logger::init();
        None
    };
    debug!("Logging initialized");

    match Cli::parse().exec() {
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
