//! A simple app that uses a VAPIX service account to access VAPIX APIs.

use std::{
    io::Read,
    process::{Command, Stdio},
    time::Duration,
};

use acap_vapix::{systemready, HttpClient};
use anyhow::{bail, Context};
use log::{debug, info};
use tokio::time;
use url::Url;

fn new_client() -> anyhow::Result<HttpClient> {
    // TODO: Consider using a DBUs library like `zbus`
    debug!("Getting credentials...");
    let mut child = Command::new("/usr/bin/gdbus")
        .arg("call")
        .arg("--system")
        .args(["--dest", "com.axis.HTTPConf1"])
        .args(["--object-path", "/com/axis/HTTPConf1/VAPIXServiceAccounts1"])
        .args([
            "--method",
            "com.axis.HTTPConf1.VAPIXServiceAccounts1.GetCredentials",
        ])
        .arg("default")
        .stdout(Stdio::piped())
        .spawn()?;
    // Unwrap is OK because `stdout` has not been taken since the child was spawned above.
    let mut stdout = child.stdout.take().unwrap();

    let status = child.wait()?;
    if !status.success() {
        bail!("Command exited with status {status}")
    }

    let mut credentials = String::new();
    stdout.read_to_string(&mut credentials)?;

    let (username, password) = credentials
        .trim()
        .strip_prefix("('")
        .context("Expected dbus response to start with ('")?
        .strip_suffix("',)")
        .context("Expected dbus response to end with ')")?
        .split_once(':')
        .context("Expected dbus response to contain at least one :")?;

    debug!("Creating client using username {username}");
    Ok(HttpClient::new(Url::parse("http://127.0.0.12")?)
        .basic_auth(username.to_string(), password.to_string()))
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    acap_logging::init_logger();
    let client = new_client().unwrap();
    loop {
        debug!("Checking if system is ready");
        let data = systemready::systemready()
            .timeout(u32::MAX)
            .execute(&client)
            .await
            .unwrap();
        if data.system_ready() {
            if let Some(uptime) = data.uptime() {
                info!("System is ready after being up for {uptime:?}");
            } else {
                info!("System is ready");
            }
            break;
        } else {
            debug!("System is not ready, checking soon.");
            time::sleep(Duration::from_secs(1)).await;
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
#[cfg(test)]
mod tests {
    use acap_vapix::systemready;

    use crate::new_client;

    #[tokio::test]
    async fn smoke_test_systemready() {
        let client = new_client().unwrap();
        let data = systemready::systemready().execute(&client).await.unwrap();
        // TODO: Remove once parsed eagerly
        let _ = data.preview_mode();
        let _ = data.uptime();
    }
}
