//! A placeholder for app specific actors
use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::{state::AppState, Message};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub foo: String,
}

// Splitting actors into an inner and outer struct make them easier to disable and, theoretically,
// to reload.
struct Inner {
    bus_tx: tokio::sync::broadcast::Sender<Message>,
    config: Config,
}

impl Inner {
    fn from_app_state(app_state: AppState) -> Option<Self> {
        let AppState {
            root_config,
            bus_tx,
            ..
        } = app_state;
        let cfg = root_config.placeholder.as_ref()?;

        Some(Self {
            bus_tx,
            config: cfg.clone(),
        })
    }
    pub async fn run(self) -> anyhow::Result<()> {
        let mut bus_rx = self.bus_tx.subscribe();
        info!("Placeholder started with config {:?}", self.config);
        // We can probably use select to wait for multiple channels at once (^_−).
        loop {
            if let Message::PlaceholderIn = bus_rx.recv().await? {
                let n = self.bus_tx.send(Message::PlaceholderOut)?;
                debug!("Placeholder sent message to {n} receivers");
            };
        }
    }
}

pub struct Placeholder {
    inner: Option<Inner>,
}

impl Placeholder {
    pub fn from_app_state(app_state: AppState) -> Self {
        Self {
            inner: Inner::from_app_state(app_state),
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        // TODO: Figure out a better way
        let Some(inner) = self.inner else {
            info!("{} not configured, sleeping...", module_path!());
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        };
        inner.run().await
    }
}
