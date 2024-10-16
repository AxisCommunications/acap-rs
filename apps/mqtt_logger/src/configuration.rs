use std::{fs, fs::File, io::BufReader};

use acap_dirs::localdata_dir;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub mqtt_broker: Option<crate::actors::mqtt_broker::Config>,
    pub mqtt_client: Option<crate::actors::mqtt_client::Config>,
}

pub fn read_config() -> anyhow::Result<AppConfig> {
    let path = localdata_dir().unwrap().join("config.json");

    // TODO: Don't use the unsafe sample config.
    if !path.exists() {
        fs::copy("sample-config.json", &path)?;
    }

    let reader = BufReader::new(File::open(path)?);
    Ok(serde_json::from_reader(reader)?)
}
