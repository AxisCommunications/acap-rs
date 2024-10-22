use std::path::PathBuf;

use clap::Parser;
use tracing::info;

use crate::database::Database;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct AbandonCommand {
    #[clap(long, default_value = "*")]
    alias: String,
}

impl AbandonCommand {
    pub async fn exec(self, file: PathBuf) -> anyhow::Result<()> {
        let mut database = Database::open_or_create(file)?;
        for alias in database.filtered_aliases(&self.alias)? {
            info!("Abandoning device {alias}...");
            let device = database.content.devices.remove(&alias).unwrap();
            device_manager::restore(&device.host, &device.primary.user, &device.primary.pass)
                .await?;
            // Update database only after the device has been restored to minimize the risk that we lose
            // track of devices with our credentials set.
            database.save()?;
        }
        Ok(())
    }
}
