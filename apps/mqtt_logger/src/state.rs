// Should be used only from main and from actors
use std::sync::Arc;

use crate::configuration::AppConfig;

#[derive(Clone)]
pub struct AppState {
    pub root_config: Arc<AppConfig>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            root_config: Arc::new(config),
        }
    }
}
