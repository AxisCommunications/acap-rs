use std::io::{BufRead, BufReader, Read};

use log::{debug, warn};

pub trait RunWith {
    fn run_with_captured_stdout(self) -> anyhow::Result<String>;
    fn run_with_processed_stdout(
        self,
        func: impl FnMut(std::io::Result<String>) -> anyhow::Result<()>,
    ) -> anyhow::Result<()>;
    fn run_with_logged_stdout(self) -> anyhow::Result<()>;
    fn run_with_inherited_stdout(self) -> anyhow::Result<()>;
}

fn spawn(mut cmd: std::process::Command) -> anyhow::Result<std::process::Child> {
    Ok(cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            let program = cmd.get_program().to_string_lossy().to_string();
            warn!("Don't forget to install {program}");
        }
        e
    })?)
}

impl RunWith for std::process::Command {
    fn run_with_captured_stdout(mut self) -> anyhow::Result<String> {
        self.stdout(std::process::Stdio::piped());
        debug!("Spawning child {self:#?}...");
        let mut child = spawn(self)?;
        let mut stdout = child.stdout.take().unwrap();
        debug!("Waiting for child...");
        let status = child.wait()?;
        if !status.success() {
            anyhow::bail!("Child failed: {status}");
        }
        let mut decoded = String::new();
        stdout.read_to_string(&mut decoded)?;
        Ok(decoded)
    }

    fn run_with_processed_stdout(
        mut self,
        mut func: impl FnMut(std::io::Result<String>) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        self.stdout(std::process::Stdio::piped());
        debug!("Spawning child {self:#?}...");
        let mut child = spawn(self)?;
        let stdout = child.stdout.take().unwrap();

        let lines = BufReader::new(stdout).lines();
        for line in lines {
            func(line)?;
        }

        debug!("Waiting for child...");
        let status = child.wait()?;
        if !status.success() {
            anyhow::bail!("Child failed: {status}");
        }
        Ok(())
    }
    fn run_with_logged_stdout(mut self) -> anyhow::Result<()> {
        self.stdout(std::process::Stdio::piped());
        debug!("Spawning child {self:#?}...");
        let mut child = spawn(self)?;
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
        if !status.success() {
            anyhow::bail!("Child failed: {status}");
        }
        Ok(())
    }

    fn run_with_inherited_stdout(mut self: std::process::Command) -> anyhow::Result<()> {
        debug!("Running command {self:#?}...");
        let status = self.status()?;
        if !status.success() {
            anyhow::bail!("Child failed: {status}");
        }
        Ok(())
    }
}
