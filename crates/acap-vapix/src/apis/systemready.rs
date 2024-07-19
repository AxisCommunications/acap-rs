//! Bindings for the [Systemready API](https://www.axis.com/vapix-library/subjects/t10175981/section/t10142629/display)
// TODO: Return actionable errors.
// TODO: Consider ignoring auth settings in client since the API does not require authentication but
//  may fail the client tries and fails to authenticate.
// TODO: Implement `getSupportedVersions`.
// TODO: Proper documentation.
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ajr_http;

const PATH: &str = "axis-cgi/systemready.cgi";
const API_VERSION: &str = "1";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "method", content = "data")]
enum Data {
    Systemready(SystemreadyData),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemreadyData {
    systemready: EnglishBool,
    needsetup: EnglishBool,
    uptime: Option<String>,
    bootid: Option<String>,
    previewmode: Option<String>,
}

impl SystemreadyData {
    // TODO: Consider renaming:
    //  - `is_ready` would be more idiomatic since the system context is already established.
    //  - `systemready` would be more in line with the API and its documentation.
    pub fn system_ready(&self) -> bool {
        self.systemready.into()
    }
    pub fn need_setup(&self) -> bool {
        self.needsetup.into()
    }

    /// Elapsed since device started?
    pub fn uptime(&self) -> Option<Duration> {
        // TODO: Ensure at parsing
        self.uptime
            .as_ref()
            .map(|t| Duration::from_secs(t.parse().unwrap()))
    }

    // A unique id generated at each boot?
    pub fn boot_id(&self) -> Option<&str> {
        self.bootid.as_deref()
    }

    /// Total or remaining?
    pub fn preview_mode(&self) -> Option<Duration> {
        // TODO: Ensure at parsing
        self.uptime
            .as_ref()
            .map(|t| Duration::from_secs(t.parse().unwrap()))
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum EnglishBool {
    Yes,
    No,
}

impl From<EnglishBool> for bool {
    fn from(value: EnglishBool) -> Self {
        match value {
            EnglishBool::Yes => true,
            EnglishBool::No => false,
        }
    }
}
#[derive(Debug)]
pub struct SystemreadyRequest {
    timeout: Option<u32>,
}

impl SystemreadyRequest {
    // TODO: Consider accepting `Duration`
    // Pros:
    // - `Duration` shows that it is a duration
    // Cons:
    // - `Duration` hides that only an integer number of seconds are considered.
    // - `Duration` hides that values bigger than `i64::MAX` are rejected.

    /// How long the server will delay the response waiting for the system to become ready.
    pub fn timeout(mut self, timeout: u32) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub async fn execute(self, client: &crate::http::Client) -> anyhow::Result<SystemreadyData> {
        let params = if let Some(timeout) = self.timeout {
            json!({"timeout":timeout})
        } else {
            json!("{}")
        };
        let Data::Systemready(data) =
            ajr_http::execute_params(PATH, API_VERSION, "systemready", params, client).await?;
        Ok(data)
    }
}
/// Please see the VAPIX Library documentation for [systemready](https://www.axis.com/vapix-library/subjects/t10175981/section/t10142629/display?section=t10142629-t10149412).
pub fn systemready() -> SystemreadyRequest {
    SystemreadyRequest { timeout: None }
}

#[cfg(test)]
mod tests {
    use crate::{ajr::ResponseEnvelope, systemready::Data};

    #[test]
    fn data_serialization_roundtrip() {
        let texts = vec![
            include_str!("systemready/10_12_initial_response.json"),
            include_str!("systemready/11_11_initial_response.json"),
        ];
        for text in texts {
            let envelope: ResponseEnvelope<Data> = serde_json::from_str(text).unwrap();
            _ = serde_json::to_string(&envelope).unwrap();
            assert!(envelope.data().is_ok());
        }
    }

    #[test]
    fn error_serialization_roundtrip() {
        let text = r#"{"apiVersion":"1.4","error":{"code":1000,"message":"Invalid JSON input"}}"#;
        let envelope: ResponseEnvelope<Data> = serde_json::from_str(text).unwrap();
        _ = serde_json::to_string(&envelope).unwrap();
        assert!(envelope.data().is_err());
    }
}
