use log::info;
use serde::{Deserialize, Serialize};

use crate::{broker::Options, state::AppState};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(flatten)]
    options: Options,
}

/// Actor that runs a mosquitto broker
struct Inner {
    _child_process: crate::broker::Broker,
}

impl Inner {
    fn from_app_state(app_state: AppState) -> Option<Self> {
        let AppState { root_config, .. } = app_state;
        let cfg = root_config.mqtt_broker.as_ref()?;
        Some(Self {
            _child_process: crate::broker::Broker::new(&cfg.options).unwrap(),
        })
    }

    async fn run(mut self) -> anyhow::Result<()> {
        tokio::time::sleep(tokio::time::Duration::new(1, 0)).await;
        while self._child_process.is_running()? {
            tokio::time::sleep(tokio::time::Duration::new(1, 0)).await;
        }
        Ok(())
    }
}

pub struct MqttBroker {
    inner: Option<Inner>,
}

impl MqttBroker {
    pub fn from_app_state(app_state: AppState) -> Self {
        Self {
            inner: Inner::from_app_state(app_state),
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let Some(inner) = self.inner else {
            info!("{} not configured, sleeping...", module_path!());
            loop {
                tokio::time::sleep(tokio::time::Duration::new(1, 0)).await;
            }
        };
        inner.run().await
    }
}
