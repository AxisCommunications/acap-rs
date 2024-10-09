//! Support for implementing bindings that use [AJR](`crate::ajr`) over HTTP.
// TODO: Return actionable error instead of `anyhow::Error`.

use std::fmt::Debug;

use anyhow::Context;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::ajr::ResponseEnvelope;

pub async fn execute_params<S, D>(
    path: &str,
    api_version: &str,
    method: &str,
    params: S,
    client: &crate::http::Client,
) -> anyhow::Result<D>
where
    S: Serialize + Debug,
    D: for<'a> Deserialize<'a>,
{
    let request_envelope = json!({
        "method": method,
        "apiVersion": api_version,
        "params": params,
    });
    execute_request(path, request_envelope, client).await
}

pub async fn execute_request<S, D>(
    path: &str,
    request_envelope: S,
    client: &crate::http::Client,
) -> anyhow::Result<D>
where
    S: Serialize + Debug,
    D: for<'a> Deserialize<'a>,
{
    // TODO: Consider not logging the request_envelope for performance and security.
    debug!("Building request from {request_envelope:?}.");
    let builder = client
        .post(path)?
        .replace_with(|b| b.json(&request_envelope));
    debug!(
        "Sending request {}",
        serde_json::to_string(&request_envelope).unwrap()
    );
    let response = builder.send().await?;
    debug!("Receiving response...");
    let status = response.status();
    let text = response.text().await?;
    // TODO: Consider not logging the text for performance and security.
    debug!("Parsing response from text {text}.");
    let response_envelope = serde_json::from_str::<ResponseEnvelope<D>>(&text)
        .context(status)
        .with_context(|| format!("Response text was {text:?}"))?;
    debug!("Convert result.");
    // TODO: Consider not logging the request for performance and security.
    response_envelope.data().with_context(|| {
        format!(
            "Request was {}",
            serde_json::to_string(&request_envelope)
                .expect("This already serialized successfully above")
        )
    })
}
