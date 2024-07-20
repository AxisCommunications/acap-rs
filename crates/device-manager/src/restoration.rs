//! A procedure for taking a device in any state to a known baseline state.
use std::time::Duration;

use acap_vapix::{systemready, HttpClient};
use anyhow::bail;
use log::{debug, info};
use tokio::time::sleep;
use url::{Host, Url};

use crate::vapix::axis_cgi::firmwaremanagement1;

enum RestartDetector<'a> {
    StateTransition(StateTransitionRestartDetector<'a>),
    Uptime(UptimeRestartDetector<'a>),
}

impl<'a> RestartDetector<'a> {
    pub async fn try_new(client: &'a HttpClient) -> anyhow::Result<Self> {
        Ok(
            match systemready::systemready().execute(client).await?.uptime() {
                Some(uptime) => Self::Uptime(UptimeRestartDetector::new(client, uptime)),
                None => Self::StateTransition(StateTransitionRestartDetector::new(client)),
            },
        )
    }

    pub async fn wait(self) -> anyhow::Result<()> {
        match self {
            Self::StateTransition(g) => g.wait().await,
            Self::Uptime(g) => g.wait().await,
        }
    }
}

enum RestartDetectorState {
    Ready,
    NotReady,
}
struct StateTransitionRestartDetector<'a> {
    client: &'a HttpClient,
    prev_state: RestartDetectorState,
}
impl<'a> StateTransitionRestartDetector<'a> {
    fn new(client: &'a HttpClient) -> Self {
        Self {
            client,
            prev_state: RestartDetectorState::Ready,
        }
    }

    async fn wait(mut self) -> anyhow::Result<()> {
        use RestartDetectorState::*;
        loop {
            let curr_state = match systemready::systemready().execute(self.client).await {
                Ok(data) => {
                    if data.system_ready() {
                        Ready
                    } else {
                        NotReady
                    }
                }
                Err(e) => {
                    debug!("Presumed not ready  because {e}");
                    NotReady
                }
            };
            self.prev_state = match (self.prev_state, curr_state) {
                (Ready, Ready) => {
                    debug!("Device is still ready");
                    Ready
                }
                (Ready, NotReady) => {
                    debug!("Device became not ready");
                    NotReady
                }
                (NotReady, NotReady) => {
                    debug!("Device is still not ready");
                    NotReady
                }
                (NotReady, Ready) => {
                    debug!("Device became ready again");
                    return Ok(());
                }
            };
            sleep(Duration::from_secs(1)).await;
        }
    }
}
struct UptimeRestartDetector<'a> {
    client: &'a HttpClient,
    // TODO: Consider using boot id when available
    uptime: Duration,
}

impl<'a> UptimeRestartDetector<'a> {
    fn new(client: &'a HttpClient, uptime: Duration) -> Self {
        Self { client, uptime }
    }

    async fn wait(mut self) -> anyhow::Result<()> {
        loop {
            match systemready::systemready().execute(self.client).await {
                Ok(data) => {
                    let uptime = data.uptime().unwrap();
                    if uptime < self.uptime {
                        debug!(
                            "Presumed restarted because uptime decreased from {:?} to {:?}",
                            self.uptime, uptime
                        );
                        return Ok(());
                    } else {
                        debug!(
                            "Presumed online still because uptime increased from {:?} to {:?}",
                            self.uptime, uptime
                        );
                        self.uptime = uptime;
                        sleep(Duration::from_secs(1)).await;
                    }
                }
                Err(e) => {
                    debug!("Presumed offline because {e}");
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}

pub async fn restore(host: &Host, user: &str, pass: &str) -> anyhow::Result<()> {
    info!("Restoring device...");
    let mut client = HttpClient::new(Url::parse(&format!("http://{host}")).unwrap());
    debug!("Checking if factory default is needed...");
    if systemready::systemready()
        .execute(&client)
        .await?
        .need_setup()
    {
        info!("Device is already in default state...");
        return Ok(());
    }

    for use_digest in [true, false] {
        if use_digest {
            info!("Trying to factory default using digest");
            client = client.digest_auth(user, pass);
        } else {
            info!("Trying to factory default using basic");
            client = client.basic_auth(user, pass);
        }

        let restart_guard = RestartDetector::try_new(&client).await?;
        match firmwaremanagement1::factory_default(
            &client,
            firmwaremanagement1::FactoryDefaultMode::Soft,
        )
        .await
        {
            Ok(()) => {
                restart_guard.wait().await?;
                return Ok(());
            }
            Err(e) => {
                debug!("Could not factory default using because {e}");
            }
        }
    }
    bail!("Could not factory default camera")
}
