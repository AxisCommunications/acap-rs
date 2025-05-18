//! Support for implementing bindings that use HTTP.

use std::{
    error::Error,
    fmt::{Debug, Display, Formatter},
};

use anyhow::{anyhow, bail};
use diqwest::WithDigestAuth;
use log::debug;
use reqwest::{Method, StatusCode};
use url::{Host, Url};

use crate::{ajr_http2::AjrHttpError, basic_device_info, systemready};

#[derive(Clone)]
struct Secret(String);

impl Debug for Secret {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "xxx")
    }
}

impl Secret {
    fn revealed(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
enum Authentication {
    Basic { username: String, password: Secret },
    Bearer { token: Secret },
    Digest { username: String, password: Secret },
    Anonymous,
}

// TODO: Expose some or all of the options available on `reqwest::ClientBuilder` keeping int mind
//  that it would be good to support curl in the future since that is available in ACAP and using
//  it may be beneficial for the footprint of apps.
/// An asynchronous client for HTTP requests.
#[derive(Clone, Debug)]
pub struct Client {
    auth: Authentication,
    base: Url,
    client: reqwest::Client,
}

impl Client {
    /// Create an HTTP client from only the host part of a URL.
    ///
    /// # Security
    ///
    /// The returned client may use HTTP, including if the server certificate is invalid.
    /// For this reason this function should not be used, except possibly during development.
    pub async fn from_host(
        host: &Host,
        http_port: Option<u16>,
        https_port: Option<u16>,
    ) -> anyhow::Result<Self> {
        // TODO: Allow users explicit control over whether to accept or reject invalid certs.
        for (scheme, port) in [("https", https_port), ("http", http_port)] {
            debug!("Trying {scheme}");
            let url = match port {
                None => &format!("{scheme}://{host}"),
                Some(port) => &format!("{scheme}://{host}:{port}"),
            };
            let url = Url::parse(url).expect("Valid schema, host and port produce a valid URL");
            let client = Self::new(url);
            if systemready::systemready()
                .execute(&client)
                .await
                .map_err(|e| debug!("{e:?}"))
                .is_ok()
            {
                return Ok(client);
            }
        }
        bail!("Could not find a scheme that works")
    }
    pub fn new(base: Url) -> Self {
        Self {
            auth: Authentication::Anonymous,
            base,
            client: reqwest::Client::new(),
        }
    }

    async fn is_authenticated(&self) -> anyhow::Result<bool> {
        let Err(e) = basic_device_info::Client::new(self)
            .get_all_properties()
            .send()
            .await
        else {
            return Ok(true);
        };
        let AjrHttpError::Transport(e) = e else {
            return Err(e.into());
        };
        if e.kind() != HttpErrorKind::Authentication {
            return Err(e.into());
        }
        Ok(false)
    }

    pub async fn automatic_auth<U, P>(self, username: U, password: P) -> anyhow::Result<Self>
    where
        U: std::fmt::Display,
        P: std::fmt::Display,
    {
        let username = username.to_string();
        let password = password.to_string();

        debug!("Trying digest authentication");
        let client = self.digest_auth(&username, &password);
        if client.is_authenticated().await? {
            return Ok(client);
        }

        debug!("Trying basic authentication");
        let client = client.basic_auth(username, password);
        if client.is_authenticated().await? {
            return Ok(client);
        }

        debug!("Trying anonymous authentication");
        let client = client.anonymous_auth();
        if client.is_authenticated().await? {
            return Ok(client);
        }

        bail!("Could not find an authentication method that works")
    }

    pub fn anonymous_auth(self) -> Self {
        Self {
            auth: Authentication::Anonymous,
            ..self
        }
    }

    pub fn basic_auth<U, P>(self, username: U, password: P) -> Self
    where
        U: std::fmt::Display,
        P: std::fmt::Display,
    {
        let username = username.to_string();
        let password = Secret(password.to_string());
        Self {
            auth: Authentication::Basic { username, password },
            ..self
        }
    }

    pub fn bearer_auth<T>(self, token: T) -> Self
    where
        T: std::fmt::Display,
    {
        let token = Secret(token.to_string());
        Self {
            auth: Authentication::Bearer { token },
            ..self
        }
    }

    /// Configure client to use digest authentication
    ///
    /// Note that this is not implemented when upgrading to websocket, and attempting to do
    /// so will return an error.
    pub fn digest_auth<U, P>(self, username: U, password: P) -> Self
    where
        U: std::fmt::Display,
        P: std::fmt::Display,
    {
        let username = username.to_string();
        let password = Secret(password.to_string());
        Self {
            auth: Authentication::Digest { username, password },
            ..self
        }
    }

    pub fn request(&self, method: Method, path: &str) -> Result<RequestBuilder, url::ParseError> {
        let mut builder = self.client.request(method, self.base.join(path)?);
        let auth = self.auth.clone();
        match &auth {
            Authentication::Basic { username, password } => {
                builder = builder.basic_auth(username, Some(password.revealed()))
            }
            Authentication::Bearer { token } => {
                builder = builder.bearer_auth(token.revealed());
            }
            Authentication::Digest { .. } => {}
            Authentication::Anonymous => {}
        }
        Ok(RequestBuilder { auth, builder })
    }

    pub fn get(&self, path: &str) -> Result<RequestBuilder, url::ParseError> {
        self.request(Method::GET, path)
    }

    pub fn post(&self, path: &str) -> Result<RequestBuilder, url::ParseError> {
        self.request(Method::POST, path)
    }

    pub fn put(&self, path: &str) -> Result<RequestBuilder, url::ParseError> {
        self.request(Method::PUT, path)
    }
}

/// The error type for executing HTTP requests.
#[derive(Debug)]
pub struct HttpError {
    inner: anyhow::Error,
    kind: HttpErrorKind,
}

impl Display for HttpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl Error for HttpError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.source()
    }
}

impl HttpError {
    pub(crate) fn other<E: Into<anyhow::Error>>(e: E) -> Self {
        Self {
            inner: e.into(),
            kind: HttpErrorKind::Other,
        }
    }

    pub(crate) fn from_status<E: Into<anyhow::Error>>(e: E, status: StatusCode) -> Self {
        let kind = match status {
            StatusCode::UNAUTHORIZED => HttpErrorKind::Authentication,
            StatusCode::FORBIDDEN => HttpErrorKind::Authorization,
            _ => HttpErrorKind::Other,
        };
        // TODO: Consider replacing the error if it can be classified
        Self {
            inner: e.into(),
            kind,
        }
    }

    fn other_from_reqwest(e: reqwest::Error) -> Self {
        // TODO: Consider demoting to `debug_assert!` or removing this helper entirely.
        assert!(e.status().is_none());
        Self::other(e)
    }

    fn other_from_reqwest_websocket(e: reqwest_websocket::Error) -> Self {
        if let reqwest_websocket::Error::Reqwest(e) = e {
            Self::other_from_reqwest(e)
        } else {
            Self::other(e)
        }
    }

    fn from_diqwest(e: diqwest::error::Error) -> Self {
        match e {
            diqwest::error::Error::Reqwest(e) => Self::other_from_reqwest(e),
            diqwest::error::Error::DigestAuth(digest_auth::Error::MissingRequired(what, ctx)) => {
                Self {
                    inner: anyhow!("Missing {what} in header: {ctx}"),
                    kind: HttpErrorKind::Authentication,
                }
            }
            diqwest::error::Error::DigestAuth(e) => Self::other(e),
            diqwest::error::Error::ToStr(e) => Self::other(e),
            diqwest::error::Error::AuthHeaderMissing => Self::other(anyhow!("auth header missing")),
            diqwest::error::Error::RequestBuilderNotCloneable => {
                Self::other(anyhow!("request builder not cloneable"))
            }
        }
    }

    /// Attempt to downcast the inner error to a concrete type.
    pub fn downcast<E: Debug + Display + Send + Sync + 'static>(self) -> Result<E, Self> {
        let Self { inner, kind } = self;
        match inner.downcast() {
            Ok(e) => Ok(e),
            Err(inner) => Err(Self { inner, kind }),
        }
    }

    /// Returns the corresponding [`HttpErrorKind`] for this error.
    pub fn kind(&self) -> HttpErrorKind {
        self.kind
    }
}

/// A list specifying categories of HTTP errors.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpErrorKind {
    // TODO: Consider collecting all status codes in one variant
    /// Corresponds to status code 401 Unauthorized.
    ///
    /// In other words, the request lacks valid credentials.
    Authentication,
    /// Corresponds to status code 403 Forbidden.
    ///
    /// In other words, request was authenticated but the credentials do not have sufficient
    /// permissions.
    Authorization,
    /// The error cannot (yet) be classified.
    ///
    /// This variant should not be used because errors of this kind are likely to be reclassified in
    /// the future.
    Other,
}

#[derive(Debug)]
pub struct RequestBuilder {
    auth: Authentication,
    builder: reqwest::RequestBuilder,
}

impl RequestBuilder {
    pub fn replace_with(
        self,
        f: impl FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    ) -> Self {
        let Self { auth, builder } = self;
        Self {
            auth,
            builder: f(builder),
        }
    }

    /// Request that the connection, once established, be upgraded to the WebSocket protocol.
    ///
    /// Note that this is not implemented when upgrading to websocket, and attempting to do
    /// so will return an error.
    pub fn upgrade(self) -> UpgradedRequestBuilder {
        use reqwest_websocket::RequestBuilderExt;

        let Self { auth, builder } = self;
        UpgradedRequestBuilder {
            auth,
            builder: builder.upgrade(),
        }
    }

    pub async fn send(self) -> Result<reqwest::Response, HttpError> {
        let Self { builder, auth } = self;
        match auth {
            Authentication::Basic { .. } => {
                builder.send().await.map_err(HttpError::other_from_reqwest)
            }
            Authentication::Bearer { .. } => {
                builder.send().await.map_err(HttpError::other_from_reqwest)
            }
            Authentication::Digest { username, password } => builder
                .send_with_digest_auth(&username, password.revealed())
                .await
                .map_err(HttpError::from_diqwest),
            Authentication::Anonymous => {
                builder.send().await.map_err(HttpError::other_from_reqwest)
            }
        }
    }
}

pub struct UpgradedRequestBuilder {
    auth: Authentication,
    builder: reqwest_websocket::UpgradedRequestBuilder,
}

impl UpgradedRequestBuilder {
    pub async fn send(self) -> Result<reqwest_websocket::UpgradeResponse, HttpError> {
        let Self { builder, auth } = self;
        match auth {
            Authentication::Basic { .. } => builder
                .send()
                .await
                .map_err(HttpError::other_from_reqwest_websocket),
            Authentication::Bearer { .. } => builder
                .send()
                .await
                .map_err(HttpError::other_from_reqwest_websocket),
            Authentication::Digest { .. } => Err(HttpError::other(anyhow!("unimplemented"))),
            Authentication::Anonymous => builder
                .send()
                .await
                .map_err(HttpError::other_from_reqwest_websocket),
        }
    }
}
