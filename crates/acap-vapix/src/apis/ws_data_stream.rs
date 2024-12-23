//! Bindings for the [Event streaming over WebSocket](https://www.axis.com/vapix-library/subjects/t10175981/section/t10195123/display) API.
// In theory websocket is able to support multiple sources each with its own set of methods.
// But such an API would be more difficult both to implement and use.
// It would nonetheless be interesting to explore how it could be mapped to a Rust API early on
// so that it can inform how other APIs are designed.
// TODO: Consider rewriting to support sources other than events.
// TODO: Consider rewriting to support multiple in flight requests.
// TODO: Return actionable error instead of `anyhow::Error`.
use std::{collections::HashMap, fmt::Display};

use anyhow::{bail, Context};
use futures_util::{sink::SinkExt, TryStreamExt};
use log::{trace, warn};
use reqwest_websocket::{Message, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    ajr2::{RequestEnvelope, ResponseEnvelope},
    HttpClient,
};

const PATH: &str = "vapix/ws-data-stream";
const API_VERSION: &str = "1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentFilter(String);

impl ContentFilter {
    pub fn unvalidated(s: impl Display) -> Self {
        Self(s.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct EventsConfigureRequest {
    event_filters: Vec<EventFilter>,
}

impl EventsConfigureRequest {
    /// Add another event filter
    ///
    /// Note that events will be sent once for each matching filter.
    pub fn event_filter<T: Into<EventFilter>>(mut self, filter: T) -> Self {
        self.event_filters.push(filter.into());
        self
    }

    pub async fn execute(self, client: &HttpClient) -> anyhow::Result<NotificationStream> {
        let response = client
            .get(PATH)?
            .replace_with(|b| b.query(&[("sources", "events")]))
            .upgrade()
            .send()
            .await?;
        let mut ws = response.into_websocket().await?;
        ws.send(Message::Text(
            serde_json::to_string(&json!({
                "apiVersion": API_VERSION,
                "method": "events:configure",
                "params": {"eventFilterList": self.event_filters}
            }))
            .unwrap(),
        ))
        .await?;

        let mut first_text: Option<String> = None;
        while let Some(message) = ws.try_next().await? {
            if let Message::Text(text) = message {
                first_text = Some(text);
                break;
            };
        }
        let first_text =
            first_text.context("websocket was closed before a response was received")?;
        let envelope: ResponseEnvelope<Value> = serde_json::from_str(&first_text)?;
        envelope.data()?;
        // TODO: Consider validating the api version and method
        Ok(NotificationStream { ws })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    content_filter: Option<ContentFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic_filter: Option<TopicFilter>,
}

impl From<ContentFilter> for EventFilter {
    fn from(value: ContentFilter) -> Self {
        Self {
            content_filter: Some(value),
            topic_filter: None,
        }
    }
}

impl From<TopicFilter> for EventFilter {
    fn from(value: TopicFilter) -> Self {
        Self {
            content_filter: None,
            topic_filter: Some(value),
        }
    }
}

impl From<(ContentFilter, TopicFilter)> for EventFilter {
    fn from((content_filter, topic_filter): (ContentFilter, TopicFilter)) -> Self {
        Self {
            content_filter: Some(content_filter),
            topic_filter: Some(topic_filter),
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationMessage {
    // TODO: Verify that these may be omitted as the docs say.
    #[serde(default)]
    pub source: HashMap<String, String>,
    #[serde(default)]
    pub key: HashMap<String, String>,
    #[serde(default)]
    pub data: HashMap<String, String>,
}

#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Notification {
    pub topic: String,
    // TODO: Verify that u64 is sufficient.
    pub timestamp: Option<u64>,
    pub message: NotificationMessage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct NotificationParams {
    notification: Notification,
}

pub struct NotificationStream {
    ws: WebSocket,
}

impl NotificationStream {
    pub async fn try_next(&mut self) -> anyhow::Result<Notification> {
        while let Some(m) = self.ws.try_next().await? {
            match m {
                Message::Text(s) => {
                    let envelope: RequestEnvelope<NotificationParams> = serde_json::from_str(&s)?;
                    return Ok(envelope.params.notification);
                }
                Message::Binary(b) => {
                    // TODO: Consider propagating this as an error instead, at least in dev.
                    warn!("Expected text, but server sent {} bytes", b.len())
                }
                Message::Ping(p) => {
                    self.ws.send(Message::Pong(p)).await?;
                }
                Message::Pong(p) => {
                    trace!("Discarding {} pong bytes", p.len())
                }
                // TODO: Consider propagating this as an error instead
                Message::Close { code, reason } => {
                    warn!("Server closed connection ({code}): {reason}")
                }
            }
        }
        bail!("Server closed connection")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopicFilter(String);

impl TopicFilter {
    // TODO: Consider implementing `From` instead.
    // Pros of `From`:
    // - Composes
    // Cons of `From`:
    // - Less obvious that the created filter may be invalid.
    pub fn unvalidated(s: impl Display) -> Self {
        Self(s.to_string())
    }
}

/// Please see the VAPIX Library documentation for [client configuration request](https://www.axis.com/vapix-library/subjects/t10175981/section/t10195123/display?section=t10195123-t10195126).
pub fn events_configure() -> EventsConfigureRequest {
    EventsConfigureRequest {
        event_filters: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_notification_request() {
        let s = r#"{"apiVersion":"1.0","method":"events:notify","params":{"notification":{"topic":"tns1:Device/tnsaxis:IO/VirtualInput","timestamp":1722108150418,"message":{"source":{"port":"38"},"key":{},"data":{"active":"0"}}}}}"#;
        let _envelope: RequestEnvelope<NotificationParams> = serde_json::from_str(s).unwrap();
    }
}
