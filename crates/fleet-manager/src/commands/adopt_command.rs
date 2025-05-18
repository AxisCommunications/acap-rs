use std::path::PathBuf;

use anyhow::bail;
use clap::Parser;
use tracing::{debug, info};
use url::Host;

use crate::database::{Account, Database, Device};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct AdoptCommand {
    /// IP Address or hostname of device to adopt.
    #[arg(long, value_parser = url::Host::parse)]
    host: Host,
    /// Override the default port for HTTP / HTTPS.
    #[clap(long, env = "AXIS_DEVICE_PORT")]
    port: Option<u16>,
    /// The username of the primary user.
    ///
    /// The primary user will be used to create any other users needed.
    /// It may also be used directly to accomplish other tasks.
    #[clap(long)]
    user: String,
    /// The password of the primary user.
    ///
    /// The primary user will be used to create any other users needed.
    /// It may also be used directly to accomplish other tasks.
    #[clap(long)]
    pass: String,
    /// Name used for filtering.
    /// Defaults to the value of `host`.
    #[clap(long)]
    alias: Option<String>,
}

impl AdoptCommand {
    pub async fn exec(self, file: PathBuf) -> anyhow::Result<()> {
        let Self {
            host,
            port,
            user,
            pass,
            alias,
        } = self;
        let alias = alias.unwrap_or_else(|| host.to_string());
        info!("Adopting device {host} as {alias}...");

        let client = match acap_vapix::HttpClient::from_host(&host, port)
            .await?
            .automatic_auth(&user, &pass)
            .await
        {
            Ok(client) => client,
            Err(e) => {
                debug!("Could not create client because {e:?}");
                info!("Could not adopt device as is, attempting to initialize it");
                device_manager::initialize(host.clone(), port, &pass).await?
            }
        };
        let arch = acap_vapix::basic_device_info::Client::new(&client)
            .get_all_properties()
            .send()
            .await?
            .property_list
            .restricted
            .architecture
            .parse()?;
        let device = Device {
            host,
            port,
            arch,
            primary: Account { user, pass },
        };
        let mut database = Database::open_or_create(file)?;
        if database.content.devices.insert(alias, device).is_some() {
            bail!("Device already adopted");
        }
        database.save()?;
        Ok(())
    }
}
