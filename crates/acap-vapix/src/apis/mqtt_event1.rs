//! Bindings for the [MQTT Event Bridge](https://www.axis.com/vapix-library/subjects/t10175981/section/t10173845/display).
// TODO: Implement remaining methods.
// TODO: Improve documentation.
// TODO: Return actionable error instead of `anyhow::Error`.
use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::ajr_http;

const PATH: &str = "axis-cgi/mqtt/event.cgi";
const API_VERSION: &str = "1.0";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "method", content = "data")]
enum Data {
    ConfigureEventPublication {},
    GetEventPublicationConfig(GetEventPublicationConfigData),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventFilter {
    topic_filter: String,
    // As of 11.11.64, explicit null is rejected by the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    qos: Option<u8>,
    // As of 11.11.64, explicit null causes server to segfault.
    #[serde(skip_serializing_if = "Option::is_none")]
    retain: Option<Retain>,
}

impl EventFilter {
    pub fn new(topic_filter: impl ToString) -> Self {
        Self {
            topic_filter: topic_filter.to_string(),
            qos: None,
            retain: Some(Retain::None),
        }
    }

    // TODO: Consider validating client side either at compile time or at runtime
    /// Accepted values are:
    /// - `0` (default)
    /// - `1`
    /// - `2`
    pub fn qos(mut self, qos: u8) -> Self {
        self.qos = Some(qos);
        self
    }

    // Default: `None`.
    pub fn retain(mut self, retain: Retain) -> Self {
        self.retain = Some(retain);
        self
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Retain {
    #[default]
    None,
    Property,
    All,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventPublicationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    topic_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_topic_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    append_event_topic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_topic_namespaces: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_serial_number_in_payload: Option<bool>,
    event_filter_list: Vec<EventFilter>,
}

/// If any properties are omitted, they will be set to their default value.
#[non_exhaustive]
pub struct ConfigureEventPublicationRequest {
    config: EventPublicationConfig,
}

impl ConfigureEventPublicationRequest {
    /// Accepted values are:
    /// - `custom`
    /// - `default` (default)
    pub fn topic_prefix(mut self, v: String) -> Self {
        self.config.topic_prefix = Some(v);
        self
    }

    /// If `topic_prefix` set to `custom` then this must be set and not empty.
    pub fn custom_topic_prefix(mut self, v: String) -> Self {
        self.config.custom_topic_prefix = Some(v);
        self
    }

    /// Default: `true`.
    pub fn append_event_topic(mut self, v: bool) -> Self {
        self.config.append_event_topic = Some(v);
        self
    }

    /// Default: `true`.
    pub fn include_topic_namespaces(mut self, v: bool) -> Self {
        self.config.include_topic_namespaces = Some(v);
        self
    }

    /// Default: `false`.
    pub fn include_serial_number_in_payload(mut self, v: bool) -> Self {
        self.config.include_serial_number_in_payload = Some(v);
        self
    }

    // Push another `EventFilter` to the `eventFilterList`.
    pub fn event_filter(mut self, event_filter: EventFilter) -> Self {
        self.config.event_filter_list.push(event_filter);
        self
    }

    pub async fn execute(self, client: &crate::http::Client) -> anyhow::Result<()> {
        let _: Data = ajr_http::execute_params(
            PATH,
            API_VERSION,
            "configureEventPublication",
            self.config,
            client,
        )
        .await?;
        Ok(())
    }
}

/// Please see the VAPIX Library documentation for [configureEventPublication](https://www.axis.com/vapix-library/subjects/t10175981/section/t10173845/display?section=t10173845-t10153066).
pub fn configure_event_publication() -> ConfigureEventPublicationRequest {
    ConfigureEventPublicationRequest {
        config: EventPublicationConfig::default(),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetEventPublicationConfigData {
    event_publication_config: EventPublicationConfig,
}

#[non_exhaustive]
pub struct GetEventPublicationConfigRequest;

impl GetEventPublicationConfigRequest {
    pub async fn execute(
        self,
        client: &crate::http::Client,
    ) -> anyhow::Result<EventPublicationConfig> {
        let data: Data =
            ajr_http::execute_params(PATH, API_VERSION, "getEventPublicationConfig", (), client)
                .await?;
        let Data::GetEventPublicationConfig(data) = data else {
            bail!("Unexpected response from server");
        };
        Ok(data.event_publication_config)
    }
}

/// Please see the VAPIX Library documentation for [getEventPublicationConfig](https://www.axis.com/vapix-library/subjects/t10175981/section/t10173845/display?section=t10173845-t10153120).
pub fn get_event_publication_config() -> GetEventPublicationConfigRequest {
    GetEventPublicationConfigRequest
}

#[cfg(test)]
mod tests {
    use crate::mqtt_event1::Data;

    #[test]
    fn can_deserialize_get_event_publication_responses() {
        // Modelling the topic prefix as an enum is problematic because:
        // - The server returns `customTopicPrefix` event when `topicPrefix` is set to "default".
        // - The server stores `customTopicPrefix` event when `topicPrefix` is set to "default".
        let text = r#"{"method":"getEventPublicationConfig","data":{"eventPublicationConfig":{"topicPrefix":"default","customTopicPrefix":"","appendEventTopic":true,"includeTopicNamespaces":true,"includeSerialNumberInPayload":false,"eventFilterList":[]}}}"#;
        let data: Data = serde_json::from_str(text).unwrap();
        println!("{data:#?}");
    }
}
