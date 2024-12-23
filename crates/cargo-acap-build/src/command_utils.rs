use std::io::{BufRead, BufReader, Read};

use anyhow::Context;
use log::debug;

pub trait RunWith {
    fn run_with_captured_stdout(self) -> anyhow::Result<String>;
    fn run_with_processed_stdout(
        self,
        func: impl FnMut(std::io::Result<String>) -> anyhow::Result<()>,
    ) -> anyhow::Result<()>;
}

fn spawn(mut cmd: std::process::Command) -> anyhow::Result<std::process::Child> {
    match cmd.spawn() {
        Ok(t) => Ok(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let program = cmd.get_program().to_string_lossy().to_string();
            Err(e).context(format!(
                "{program} not found, perhaps it must be installed."
            ))
        }
        Err(e) => Err(e.into()),
    }
}

impl RunWith for std::process::Command {
    fn run_with_captured_stdout(mut self) -> anyhow::Result<String> {
        self.stdout(std::process::Stdio::piped());
        debug!("Spawning child {self:#?}...");
        let mut child = spawn(self)?;
        let mut stdout = child.stdout.take().unwrap();
        let mut decoded = String::new();
        stdout.read_to_string(&mut decoded)?;
        debug!("Waiting for child...");
        let status = child.wait()?;
        if !status.success() {
            anyhow::bail!("Child failed: {status}");
        }
        Ok(decoded)
    }

    fn run_with_processed_stdout(
        mut self,
        mut func: impl FnMut(std::io::Result<String>) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        self.stdout(std::process::Stdio::piped());
        debug!("Spawning child {self:#?}...");
        let mut child = spawn(self)?;
        let stdout = child
            .stdout
            .take()
            .expect("not previously taken by this function");

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
}
