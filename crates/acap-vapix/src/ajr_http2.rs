//! Support for implementing bindings that use [AJR](`crate::ajr`) over HTTP.

use std::{
    error::Error,
    fmt::{Debug, Display, Formatter},
};

use log::debug;
use serde::{Deserialize, Serialize};

use crate::{
    ajr,
    ajr2::ResponseEnvelope,
    http::{HttpError, HttpErrorKind},
};

// Auth, or at least authorization, errors can be communicate using either or both of AJR and HTTP.
// If this and branching on auth errors is common, it may be convenient to lift them out so that
// users don't have to inspect two variants.
// TODO: Consider giving auth errors their own category
#[derive(Debug)]
pub enum AjrHttpError {
    // TODO: Consider using something more general to allow request building to fail in other ways.
    Build(url::ParseError),
    Transport(HttpError),
    Procedure(ajr::Error),
}

impl Display for AjrHttpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Build(e) => Display::fmt(e, f),
            Self::Transport(e) => Display::fmt(e, f),
            Self::Procedure(e) => Display::fmt(e, f),
        }
    }
}

impl Error for AjrHttpError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self {
            AjrHttpError::Build(e) => Some(e),
            AjrHttpError::Transport(e) => Some(e),
            AjrHttpError::Procedure(e) => Some(e),
        }
    }
}

impl From<ajr::Error> for AjrHttpError {
    fn from(value: ajr::Error) -> Self {
        Self::Procedure(value)
    }
}

impl From<HttpError> for AjrHttpError {
    fn from(value: HttpError) -> Self {
        match value.kind() {
            HttpErrorKind::Authentication => Self::Transport(value),
            HttpErrorKind::Authorization => Self::Transport(value),
            HttpErrorKind::Other => Self::Transport(value),
        }
    }
}

impl AjrHttpError {
    fn build(e: url::ParseError) -> Self {
        Self::Build(e)
    }
}

pub async fn execute_request<S, D>(
    path: &str,
    request_envelope: S,
    client: &crate::http::Client,
) -> Result<D, AjrHttpError>
where
    S: Serialize + Debug,
    D: for<'a> Deserialize<'a>,
{
    // TODO: Consider not logging the request_envelope for performance and security.
    debug!("Building request from {request_envelope:?}.");
    let builder = client
        .post(path)
        .map_err(AjrHttpError::build)?
        .replace_with(|b| b.json(&request_envelope));
    debug!(
        "Sending request {}",
        serde_json::to_string(&request_envelope).unwrap()
    );
    let response = builder.send().await?;
    debug!("Receiving response...");
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| HttpError::from_status(e, status))?;
    // TODO: Consider not logging the text for performance and security.
    debug!("Parsing response from text {text}.");
    let response_envelope = serde_json::from_str::<ResponseEnvelope<D>>(&text)
        .map_err(|e| HttpError::from_status(e, status))?;
    debug!("Convert result.");
    Ok(response_envelope.data()?)
}
