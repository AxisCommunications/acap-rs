//! A simple app that uses a VAPIX service account to access VAPIX APIs.

use std::{
    io::Read,
    process::{Command, Stdio},
    time::Instant,
};

use acap_vapix::{parameter_management, HttpClient};
use anyhow::{bail, Context};
use log::{debug, info};
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
    let before = Instant::now();
    let params = parameter_management::list().execute(&client).await.unwrap();
    let after = Instant::now();
    debug!(
        "Retrieving parameters took {} milliseconds",
        after.duration_since(before).as_millis()
    );
    info!("Retrieved {} parameters", params.len())
}

#[cfg(not(target_arch = "x86_64"))]
#[cfg(test)]
mod tests {
    use acap_vapix::{certificate_management, mqtt_client1, mqtt_event1, parameter_management};
    use serde_json::Value;

    use crate::new_client;

    #[tokio::test]
    async fn smoke_test_certificate_management() {
        let client = new_client().unwrap();
        assert!(
            certificate_management::delete_certificates("nonexistent-certificate-id")
                .execute(&client)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn smoke_test_mqtt_client1() {
        let client = new_client().unwrap();
        let data = mqtt_client1::get_client_status()
            .execute(&client)
            .await
            .unwrap();
        let Some(Value::Object(status)) = data.get("status") else {
            panic!("Status seems like something that should always be present")
        };
        let Some(Value::String(state)) = status.get("state") else {
            panic!("State seems like something that should always be present")
        };
        assert!(state == "active" || state == "inactive")
    }

    #[tokio::test]
    async fn smoke_test_mqtt_event1() {
        let client = new_client().unwrap();
        let _ = mqtt_event1::get_event_publication_config()
            .execute(&client)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn smoke_test_parameter_management() {
        let client = new_client().unwrap();
        let params = parameter_management::list().execute(&client).await.unwrap();
        // This is not a documented invariant, but it seems unlikely to change, and it is useful for
        // verifying that the parameters were retrieved somewhat correctly.
        assert_eq!(params.get("root.Brand.Brand").unwrap(), "AXIS")
    }
}
