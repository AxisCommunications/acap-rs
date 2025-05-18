use std::{
    io::{BufRead, BufReader},
    panic,
    path::PathBuf,
    process::ExitStatus,
    thread::spawn,
};

use anyhow::{bail, Context};
use clap::Parser;
use tracing::{debug, span, Level};

use crate::database::Database;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ForEachCommand {
    /// Glob pattern specifying which devices to operate on.
    #[clap(long, default_value = "*")]
    alias: String,
    /// Run the command for each device in its own thread.
    #[clap(short, long, default_value = "false")]
    parallel: bool,
    /// Program and arguments to run for each specified device.
    command: Vec<String>,
}

impl ForEachCommand {
    pub fn exec(self, file: PathBuf) -> anyhow::Result<()> {
        let Self {
            alias: host,
            parallel,
            command,
        } = self;
        let database = Database::open_or_create(file)?;
        let mut commands = Vec::new();
        for alias in database.filtered_aliases(&host)? {
            let device = database.content.devices.get(&alias).unwrap();
            let mut cmd = std::process::Command::new(
                command
                    .first()
                    .context("command must at least specify a program")?,
            );
            cmd.args(command.iter().skip(1));
            cmd.env("AXIS_DEVICE_IP", device.host.to_string());
            if let Some(port) = device.port {
                cmd.env("AXIS_DEVICE_PORT", port.to_string());
            }
            cmd.env("AXIS_DEVICE_ARCH", device.arch.nickname());
            cmd.env("AXIS_DEVICE_USER", &device.primary.user);
            cmd.env("AXIS_DEVICE_PASS", &device.primary.pass);
            commands.push((alias, cmd));
        }

        if parallel {
            let mut results = Vec::new();
            let handles = commands
                .into_iter()
                .map(|(h, c)| spawn(move || log_stdout(c, &h)))
                .collect::<Vec<_>>();
            for h in handles {
                match h.join() {
                    Ok(r) => results.push(r),
                    Err(e) => panic::resume_unwind(e),
                }
            }
            if results.iter().any(|r| r.is_err()) {
                bail!("Some commands failed");
            }
        } else {
            for (_, mut c) in commands {
                assert!(c.status()?.success());
            }
        };

        Ok(())
    }
}

fn log_stdout(mut cmd: std::process::Command, name: &str) -> anyhow::Result<ExitStatus> {
    let span = span!(Level::DEBUG, "", device = name).entered();

    cmd.stdout(std::process::Stdio::piped());
    debug!("Spawning child {cmd:#?}...");
    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().unwrap();

    let lines = BufReader::new(stdout).lines();
    for line in lines {
        let line = line?;
        if !line.is_empty() {
            debug!("Child said {:?}.", line);
        }
    }

    debug!("Waiting for child...");
    let status = child.wait()?;
    span.exit();
    Ok(status)
}
