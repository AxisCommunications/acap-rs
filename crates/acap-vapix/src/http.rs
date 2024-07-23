//! Support for implementing bindings that use HTTP.
use std::fmt::{Debug, Formatter};

use anyhow::bail;
use diqwest::WithDigestAuth;
use reqwest::Method;
use url::Url;

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
    pub fn new(base: Url) -> Self {
        Self {
            auth: Authentication::Anonymous,
            base,
            client: reqwest::Client::new(),
        }
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

    pub async fn send(self) -> anyhow::Result<reqwest::Response> {
        let Self { builder, auth } = self;
        match auth {
            Authentication::Basic { .. } => Ok(builder.send().await?),
            Authentication::Bearer { .. } => Ok(builder.send().await?),
            Authentication::Digest { username, password } => Ok(builder
                .send_with_digest_auth(&username, password.revealed())
                .await?),
            Authentication::Anonymous => Ok(builder.send().await?),
        }
    }
}

pub struct UpgradedRequestBuilder {
    auth: Authentication,
    builder: reqwest_websocket::UpgradedRequestBuilder,
}

impl UpgradedRequestBuilder {
    pub async fn send(self) -> anyhow::Result<reqwest_websocket::UpgradeResponse> {
        let Self { builder, auth } = self;
        match auth {
            Authentication::Basic { .. } => Ok(builder.send().await?),
            Authentication::Bearer { .. } => Ok(builder.send().await?),
            Authentication::Digest { .. } => bail!("unimplemented"),
            Authentication::Anonymous => Ok(builder.send().await?),
        }
    }
}
