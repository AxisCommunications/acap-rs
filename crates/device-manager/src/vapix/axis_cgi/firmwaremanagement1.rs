use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::vapix::ajr_http;

const PATH: &str = "axis-cgi/firmwaremanagement.cgi";
const VERSION: &str = "1.0";
pub async fn factory_default(
    client: &acap_vapix::HttpClient,
    mode: FactoryDefaultMode,
) -> anyhow::Result<()> {
    let data = ajr_http::exec(
        client,
        PATH,
        VERSION,
        None,
        Params::FactoryDefault(FactoryDefaultParams {
            factory_default_mode: mode,
        }),
    )
    .await?;
    let Data::FactoryDefault {} = data else {
        bail!("Expected Data::FactoryDefault but got {data:?}")
    };
    Ok(())
}
#[derive(Debug, Serialize, Deserialize)]
pub struct DataResponse {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "method", content = "params")]
enum Params {
    FactoryDefault(FactoryDefaultParams),
    Status,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FactoryDefaultParams {
    factory_default_mode: FactoryDefaultMode,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FactoryDefaultMode {
    Soft,
    Hard,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "method", content = "data")]
enum Data {
    FactoryDefault {},
    Status(StatusData),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusData {
    /// Current firmware version.
    active_firmware_version: String,
    /// Current firmware part number.
    active_firmware_part: String,
    /// Inactive firmware version. This is only present if an inactive firmware exists, which will
    /// be reported as “UNKNOWN” if the inactive firmware doesn't support the automatic firmware
    /// rollback parameters.
    inactive_firmware_version: Option<String>,
    /// True if current firmware is committed.
    /// False if the current firmware is uncommitted and will rollback on reboot.
    ///
    /// This is only present if an inactive firmware exists.
    is_commited: Option<bool>,
    /// Pending auto commit.
    ///
    /// "started" The current firmware will be automatically committed once the device has finished
    /// booting, see Upgrade.
    ///
    /// This is only present if the active firmware is uncommitted and an automatic commit is
    /// pending.
    pending_commit: Option<String>,
    /// Number of seconds left to automatic rollback.
    ///
    /// This is only present if active firmware is uncommitted and an automatic rollback is pending.
    time_to_rollback: Option<u32>,
    /// The date and time when the Axis product was upgraded.
    ///
    /// This is only present if an inactive firmware exists.
    last_upgrade_at: Option<String>,
}
