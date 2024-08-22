//! Bindings for the [MQTT client API](https://www.axis.com/vapix-library/subjects/t10175981/section/t10152603/display).
// TODO: Implement remaining methods.
// TODO: Improve documentation.
// TODO: Return actionable error instead of `anyhow::Error`.
use anyhow::bail;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::ajr_http;

const PATH: &str = "axis-cgi/mqtt/client.cgi";
const API_VERSION: &str = "1.0";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "method", content = "data")]
enum Data {
    ActivateClient {},
    ConfigureClient {},
    GetClientStatus(GetClientStatusData),
}

#[non_exhaustive]
pub struct ActivateClientRequest;

impl ActivateClientRequest {
    pub async fn execute(self, client: &crate::http::Client) -> anyhow::Result<()> {
        let data: Data =
            ajr_http::execute_params(PATH, API_VERSION, "activateClient", json!({}), client)
                .await?;
        let Data::ActivateClient {} = data else {
            bail!("Server responded with incorrect method")
        };
        Ok(())
    }
}

pub fn activate_client() -> ActivateClientRequest {
    ActivateClientRequest
}

#[non_exhaustive]
pub struct ConfigureClientRequest {
    params: Value,
}

impl ConfigureClientRequest {
    pub fn replace_with(self, f: impl FnOnce(Value) -> Value) -> Self {
        Self {
            params: f(self.params),
        }
    }
    pub async fn execute(self, client: &crate::http::Client) -> anyhow::Result<()> {
        let data: Data =
            ajr_http::execute_params(PATH, API_VERSION, "configureClient", self.params, client)
                .await?;
        let Data::ConfigureClient {} = data else {
            bail!("Server responded with incorrect method")
        };
        Ok(())
    }
}

pub fn configure_client() -> ConfigureClientRequest {
    ConfigureClientRequest {
        params: Value::default(),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetClientStatusData(Map<String, Value>);

impl GetClientStatusData {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }
}

#[non_exhaustive]
pub struct GetClientStatusRequest;

impl GetClientStatusRequest {
    pub async fn execute(
        self,
        client: &crate::http::Client,
    ) -> anyhow::Result<GetClientStatusData> {
        let data: Data =
            ajr_http::execute_params(PATH, API_VERSION, "getClientStatus", json!({}), client)
                .await?;
        let Data::GetClientStatus(data) = data else {
            bail!("Server responded with incorrect method")
        };
        Ok(data)
    }
}

pub fn get_client_status() -> GetClientStatusRequest {
    GetClientStatusRequest
}
