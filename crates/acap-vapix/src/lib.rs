#![doc = include_str!("../README.md")]

use std::{
    io::Read,
    process::{Command, Stdio},
};

use anyhow::{bail, Context};
pub use apis::systemready;
pub use http::Client as HttpClient;
use log::debug;
use url::Url;

mod ajr;
mod ajr_http;
mod apis;
mod http;

/// Construct a new [`HttpClient`] for ACAP apps connecting to VAPIX on the same device.
pub fn local_client() -> anyhow::Result<HttpClient> {
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
