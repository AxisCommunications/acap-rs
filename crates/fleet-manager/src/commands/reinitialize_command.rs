use std::path::PathBuf;

use clap::Parser;
use tracing::info;

use crate::database::Database;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ReinitializeCommand {
    /// Glob pattern specifying which devices to operate on.
    #[clap(long, default_value = "*")]
    alias: String,
}

impl ReinitializeCommand {
    pub async fn exec(self, file: PathBuf) -> anyhow::Result<()> {
        let mut database = Database::open_or_create(file)?;
        for alias in database.filtered_aliases(&self.alias)? {
            info!("Reinitializing device {alias}...");
            let device = database.content.devices.get_mut(&alias).unwrap();
            device_manager::restore(
                &device.host,
                device.http_port,
                device.https_port,
                &device.primary.user,
                &device.primary.pass,
            )
            .await?;
            device_manager::initialize(device.host.clone(), device.http_port, &device.primary.pass)
                .await?;
            device.primary.user = "root".to_string();
        }
        Ok(())
    }
}
