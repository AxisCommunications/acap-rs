use std::{
    fs,
    fs::File,
    io::{BufReader, Write},
};

use acap_dirs::localdata_dir;
use log::info;
use serde::{Deserialize, Serialize};

use crate::actors::{placeholder, server};

/// Enable by default and allow users to _opt out_.
fn opt_out<T: Default>() -> Option<T> {
    Some(T::default())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppConfig {
    // TODO: Reason about whether the config should be DRY
    // DRY is less verbose and prone to desync
    // but it also increases coupling.
    #[serde(default = "opt_out")]
    pub placeholder: Option<placeholder::Config>,
    #[serde(default)]
    pub web: server::Config,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            placeholder: Some(placeholder::Config::default()),
            web: server::Config::default(),
        }
    }
}

pub fn read_config() -> AppConfig {
    let Ok(file) = File::open(localdata_dir().unwrap().join("config.json")) else {
        info!("No config file found, using default config");
        return AppConfig::default();
    };
    let reader = BufReader::new(file);
    let result = serde_json::from_reader(reader).unwrap();
    info!("Loaded config: {:?}", result);
    result
}

pub fn write_config(config: &AppConfig) -> anyhow::Result<()> {
    let committed = localdata_dir().unwrap().join("config.json");
    let staged = localdata_dir().unwrap().join("config.json.tmp");
    let mut file = File::create(&staged)?;
    let content = serde_json::to_string(config)?;
    file.write_all(content.as_bytes())?;
    fs::rename(staged, committed)?;
    Ok(())
}
