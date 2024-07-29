use std::time::Duration;

use anyhow::bail;
use futures::stream::StreamExt;
use log::{info, warn};
use serde::{Deserialize, Serialize};

use crate::{client::Options, state::AppState};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(flatten)]
    options: Options,
}

// TODO: Figure out how to send and receive
// (probably want to replace the broadcast channel with a callback system)
/// Actor that runs a mosquitto broker and relays messages
struct Inner {
    client_options: Options,
}

impl Inner {
    fn from_app_state(app_state: AppState) -> Option<Self> {
        let AppState { root_config, .. } = app_state;
        let cfg = root_config.mqtt_client.as_ref()?;

        Some(Self {
            client_options: cfg.options.clone(),
        })
    }

    async fn run(self) -> anyhow::Result<()> {
        // TODO: More robust way to ensure that the consumer connects if the broker is temporarily
        // unavailable.
        tokio::time::sleep(Duration::new(1, 0)).await;
        let mut client = crate::client::Client::new(&self.client_options).await?;
        let client = client.get_mut();
        client.subscribe("#", 2).await?;

        let mut stream = client.get_stream(25);

        while let Some(msg_opt) = stream.next().await {
            if let Some(msg) = msg_opt {
                info!("topic: {topic}; payload: {payload};",topic=msg.topic(), payload=msg.payload_str());
            } else {
                warn!("Connection lost, reconnecting");
                while let Err(e) = client.reconnect().await {
                    warn!("Reconnect failed because, retrying shortly: {}", e);
                    tokio::time::sleep(Duration::new(1, 0)).await;
                }
            }
        }
        bail!("Stream ended");
    }
}

pub struct MqttClient {
    inner: Option<Inner>,
}

impl MqttClient {
    pub fn from_app_state(app_state: AppState) -> Self {
        Self {
            inner: Inner::from_app_state(app_state),
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let Some(inner) = self.inner else {
            info!("{} not configured, sleeping...", module_path!());
            loop {
                tokio::time::sleep(Duration::new(1, 0)).await;
            }
        };
        inner.run().await
    }
}
