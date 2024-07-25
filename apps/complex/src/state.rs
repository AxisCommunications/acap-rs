use std::sync::{Arc, Mutex};

// Should be used only from main and from actors
use crate::configuration::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub root_config: Arc<AppConfig>,
    pub bus_tx: tokio::sync::broadcast::Sender<crate::Message>,
    pub stop_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    pub stop_rx: Arc<Mutex<Option<tokio::sync::oneshot::Receiver<()>>>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let (bus_tx, _) = tokio::sync::broadcast::channel(100);
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        Self {
            root_config: Arc::new(config),
            bus_tx,
            stop_tx: Arc::new(Mutex::new(Some(stop_tx))),
            stop_rx: Arc::new(Mutex::new(Some(stop_rx))),
        }
    }
}
