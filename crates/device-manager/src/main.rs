use std::{env, fs::File};

use clap::{Parser, Subcommand};
use device_manager::{initialize, restore};
use log::{debug, info};
use url::Host;

/// Utilities for managing individual devices.
#[derive(Clone, Debug, Parser)]
#[clap(verbatim_doc_comment)]
struct Cli {
    #[command(flatten)]
    netloc: Netloc,
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    async fn exec(self) -> anyhow::Result<()> {
        let Self {
            netloc: Netloc { host, user, pass },
            command,
        } = self;
        match command {
            Command::Reinit => {
                restore(&host, &user, &pass).await?;
                initialize(host, &pass).await?;
                if user != "root" {
                    println!("Remember that the primary user has changed from {user} to root")
                }
            }
            Command::Restore => {
                restore(&host, &user, &pass).await?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Parser)]
struct Netloc {
    /// Hostname or IP address of the device.
    #[arg(long, value_parser = url::Host::parse, env = "AXIS_DEVICE_IP")]
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
    /// Restore device to a clean state.
    Restore,
    /// Restore and initialize device to a known, useful state.
    Reinit,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    // There are probably many places where this program could get stuck, such as when waiting for
    // a parameter to change, and even when it succeeds it takes a long time. Interrupting it causes
    // it to exit without writing logs to disk, so if they were not printed to stderr information
    // about where the program was interrupted is lost. This makes it harder to find and report
    // problems.
    // TODO: Find and bound unbounded retry loops
    // TODO: Save logs on SIGINT.
    match Cli::parse().exec().await {
        Ok(()) => {
            info!("Orl Korrect");
            Ok(())
        }
        Err(e) => {
            if let Some(log_file) = log_file {
                Err(e.context(format!("A detailed log has been saved to {log_file:?}")))
            } else {
                Err(e)
            }
        }
    }
}
