//! A simple app that uses a VAPIX service account to access VAPIX APIs.

use std::time::Duration;

use acap_vapix::systemready;
use log::{debug, info};
use tokio::time;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    acap_logging::init_logger();
    let client = acap_vapix::local_client().unwrap();
    loop {
        debug!("Checking if system is ready");
        let data = systemready::systemready().execute(&client).await.unwrap();
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

    #[tokio::test]
    async fn smoke_test_systemready() {
        let client = acap_vapix::local_client().unwrap();
        let data = systemready::systemready().execute(&client).await.unwrap();
        // TODO: Remove once parsed eagerly
        let _ = data.preview_mode();
        let _ = data.uptime();
    }
}
