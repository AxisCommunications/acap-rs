#![doc = include_str!("../README.md")]

use std::{
    env,
    io::Read,
    process::{Command, Stdio},
};

use anyhow::{bail, Context};
pub use apis::{
    applications_control, applications_upload, basic_device_info, parameter_management,
    systemready, ws_data_stream,
};
pub use http::{Client as HttpClient, HttpError, HttpErrorKind};
use log::debug;
use url::Url;

mod ajr;
mod ajr2;
mod ajr_http;
mod ajr_http2;
mod apis;
mod http;

fn from_dbus() -> anyhow::Result<HttpClient> {
    // TODO: Consider verifying the manifest or providing hints when it looks misconfigured and this
    //  call fails.
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
        // TODO: Consider capturing stderr and attaching to error or logging as warning.
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
    debug!("Creating client using username {username} from dbus");
    Ok(
        HttpClient::new(Url::parse("http://127.0.0.12").expect("Hardcoded url is valid"))
            .basic_auth(username, password),
    )
}

fn from_env() -> anyhow::Result<HttpClient> {
    let username = env::var("AXIS_DEVICE_USER")?;
    let password = env::var("AXIS_DEVICE_PASS")?;
    let host = env::var("AXIS_DEVICE_IP")?;
    let url = Url::parse(&format!("http://{host}"))?;
    debug!("Creating client using username {username} from env");
    // TODO: Select appropriate authentication scheme
    // When connecting locally basic is always used but when connecting remotely the default is
    // currently digest, so it would be convenient if that was applied to the client.
    Ok(HttpClient::new(url).basic_auth(username, password))
}

/// Construct a new [`HttpClient`] for ACAP apps connecting to VAPIX.
pub fn local_client() -> anyhow::Result<HttpClient> {
    // TODO: Find a more robust configuration
    if cfg!(target_arch = "x86_64") {
        from_env()
    } else {
        from_dbus()
    }
}
