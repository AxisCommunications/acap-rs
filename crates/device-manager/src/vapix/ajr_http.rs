use std::fmt::Debug;

use acap_vapix::HttpClient;
use anyhow::Context;
use log::debug;
use serde::{Deserialize, Serialize};

use crate::vapix::{
    ajr,
    ajr::{RequestEnvelope, ResponseEnvelope},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Transport(reqwest::Error),
    // A valid HTTP response is received but we cannot parse an AJR response because
    // * not valid json
    // * does not deserialize into the typed ResponseEnvelope
    // * the data is the wrong variant
    #[error(transparent)]
    Protocol(anyhow::Error),
    #[error(transparent)]
    Application(ajr::Error),
    // TODO: Remove
    #[error(transparent)]
    Other(anyhow::Error),
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::Transport(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Protocol(value.into())
    }
}
impl From<ajr::Error> for Error {
    fn from(value: ajr::Error) -> Self {
        Self::Application(value)
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self::Other(value)
    }
}

/// Use `client` to execute RPC specified by `path`, `api_version`, `method`, and `params`
pub async fn exec<P, D>(
    client: &HttpClient,
    path: &str,
    // Strictly this should be an int for the major because APIs server only one minor at a time.
    // But some APIs require a minor version too.
    api_version: impl ToString,
    // Strictly this should always be None because it is redundant for HTTP, which is already
    // request-response, making the overhead unjustified.
    // But some APIs require it anyway.
    context: Option<String>,
    params: P,
) -> Result<D, Error>
where
    P: Serialize + Debug,
    D: for<'a> Deserialize<'a>,
{
    let request_envelope = RequestEnvelope::new(api_version, context, params);
    debug!("Building from {request_envelope:#?}");
    debug!(
        "Text: {}",
        serde_json::to_string(&request_envelope).unwrap()
    );
    let builder = client
        .post(path)
        .unwrap()
        .replace_with(|b| b.json(&request_envelope));
    debug!("Sending");
    let http = builder.send().await?;
    debug!("Extracting");
    let status = http.status();
    let text = http.text().await?;
    debug!("Response text: {text}");
    let response_envelope = serde_json::from_str::<ResponseEnvelope<D>>(&text).context(status)?;
    Ok(response_envelope.data()?)
}
