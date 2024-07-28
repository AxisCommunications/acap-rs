//! Support for implementing bindings that use Axis JSON RPC (AJR), regardless of transport.
//!
//! This module is independent of how the RPCs are transported. Transport specific utilities are
//! provided by separate modules:
//! - [`crate::ajr_http`]
// TODO: Consider making errors generic with a default.
// TODO: Merge with `ajr` or develop a clear story and language for what each does.
use serde::{Deserialize, Serialize};

use crate::ajr::Error;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestEnvelope<T> {
    api_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
    method: String,
    pub params: T,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseEnvelope<T> {
    api_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
    method: Option<String>,
    #[serde(flatten)]
    result: TaggedResult<T>,
}

impl<T> ResponseEnvelope<T> {
    pub fn data(self) -> Result<T, Error> {
        match self.result {
            TaggedResult::Data(d) => Ok(d),
            TaggedResult::Error(e) => Err(e),
        }
    }
}

// TODO: Consider writing custom (de)serializers for `ResponseEnvelope` instead.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TaggedResult<T> {
    // It may be possible to improve performance by using `RawValue` instead of `Value`.
    Data(T),
    Error(Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn can_parse_empty_data_response() {
        let s = r#"{"apiVersion":"1.0","method":"events:configure","data":{}}"#;
        let envelope: ResponseEnvelope<()> = serde_json::from_str(s).unwrap();
        envelope.data().unwrap();
    }
    #[test]
    #[ignore]
    #[should_panic]
    /// Documents the fact that this implementation is incapable of (de)serializing a responses with
    /// no member named either _data_ or _error_, which some VAPIX APIs use to signal that a request
    /// was successful.
    fn can_parse_missing_data_response() {
        let s = r#"{"apiVersion":"1.0","method":"events:configure"}"#;
        let envelope: ResponseEnvelope<()> = serde_json::from_str(s).unwrap();
        envelope.data().unwrap();
    }
}
