//! A simple app that uses VAPIX access

use std::{
    collections::HashMap,
    env,
    io::Read,
    process::{Command, Stdio},
    time::Instant,
};

use log::{debug, info};

// TODO: Improve error handling
async fn list_parameters() -> HashMap<String, String> {
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
        .arg("testuser")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let status = child.wait().unwrap();
    assert!(status.success());
    let mut credentials = String::new();
    stdout.read_to_string(&mut credentials).unwrap();
    let (username, password) = credentials
        .trim()
        .strip_prefix("('")
        .unwrap()
        .strip_suffix("',)")
        .unwrap()
        .split_once(':')
        .unwrap();

    debug!("Got credentials for username {username}");

    debug!("Using credentials to retrieve parameters...");
    let client = reqwest::Client::new();
    let before = Instant::now();
    let response = client
        .get("http://127.0.0.12/axis-cgi/param.cgi?action=list")
        .basic_auth(username, Some(password))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let after = Instant::now();
    debug!(
        "Retrieving parameters took {} milliseconds",
        after.duration_since(before).as_millis()
    );
    response
        .trim()
        .lines()
        .map(|l| {
            let (k, v) = l.split_once('=').unwrap();
            (k.to_string(), v.to_string())
        })
        .collect()
}

#[tokio::main]
async fn main() {
    app_logging::init_logger();
    let params = list_parameters().await;
    info!("Retrieved {} parameters", params.len())
}

#[cfg(not(target_arch = "x86_64"))]
#[cfg(test)]
mod tests {
    use crate::list_parameters;

    #[tokio::test]
    async fn can_retrieve_parameters() {
        let params = list_parameters().await;
        // This is not a documented invariant, but it seems unlikely to change, and it is useful for
        // verifying that the parameters were retrieved somewhat correctly.
        assert_eq!(params.get("root.Brand.Brand").unwrap(), "AXIS")
    }
}
