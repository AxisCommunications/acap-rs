#![forbid(unsafe_code)]
use std::{ffi::OsString, path::PathBuf};

use anyhow::Context;
use clap::{Parser, Subcommand};
use cli_version::version_with_commit_id;
use tracing::debug;

use crate::commands::{
    abandon_command::AbandonCommand, adopt_command::AdoptCommand, for_each_command::ForEachCommand,
    reinitialize_command::ReinitializeCommand,
};

mod commands;
mod database;

// TODO: Considering removing all device interaction from this program.

/// Utilities for managing devices in bulk.
#[derive(Debug, Parser)]
#[clap(verbatim_doc_comment, version = version_with_commit_id!())]
struct Cli {
    /// Location of database file.
    fleet: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

fn program_name() -> anyhow::Result<OsString> {
    Ok(std::env::current_exe()?
        .file_name()
        .context("file_name")?
        .to_os_string())
}

impl Cli {
    async fn exec(self) -> anyhow::Result<()> {
        let Self { fleet, command } = self;
        let fleet = match fleet {
            None => dirs::data_dir()
                .context("data dir")?
                .join(program_name()?)
                .join("devices.json"),
            Some(custom) => custom,
        };
        match command {
            // Core
            Command::Adopt(cmd) => cmd.exec(fleet).await,
            Command::Abandon(cmd) => cmd.exec(fleet).await,
            // Generic
            Command::ForEach(cmd) => cmd.exec(fleet),
            // Bespoke
            Command::Reinit(cmd) => cmd.exec(fleet).await,
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    // Core commands
    /// Add a device to the chosen fleet.
    Adopt(AdoptCommand),
    /// Restore selected device(s) and remove from the chosen fleet.
    Abandon(AbandonCommand),

    // Generic commands enabling a wide range of use cases
    /// Run the provided command on selected devices, in parallel or in sequence.
    ForEach(ForEachCommand),

    // Bespoke commands leveraging tight integration for a better user experience
    /// Restore and initialize selected device(s) to a known, useful state.
    Reinit(ReinitializeCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    debug!("Logging initialized");
    Cli::parse().exec().await
}
