//! Bindings for [part of the Application API](https://www.axis.com/vapix-library/subjects/t10102231/section/t10036126/display?section=t10036126-t10010609).
// TODO: Return actionable errors.
// TODO: Proper documentation.

use std::{
    error::Error,
    fmt::{Debug, Display, Formatter},
    fs, io,
    path::Path,
    str::FromStr,
};

use reqwest::{
    multipart::{Form, Part},
    StatusCode,
};

use crate::HttpClient;

pub const PATH: &str = "axis-cgi/applications/upload.cgi";

#[derive(Debug)]
pub enum HttpRpcError<T> {
    Remote(T),
    ParseUrl(url::ParseError),
    Other(anyhow::Error),
}

impl<T: Display> Display for HttpRpcError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpRpcError::Remote(e) => Display::fmt(&e, f),
            HttpRpcError::ParseUrl(e) => Display::fmt(&e, f),
            HttpRpcError::Other(e) => Display::fmt(&e, f),
        }
    }
}

impl<T: Error + 'static> Error for HttpRpcError<T> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            HttpRpcError::Remote(e) => Some(e),
            HttpRpcError::ParseUrl(e) => Some(e),
            HttpRpcError::Other(e) => e.source(),
        }
    }
}

impl<T> HttpRpcError<T> {
    fn other(e: impl Into<anyhow::Error>) -> Self {
        Self::Other(e.into())
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub enum UploadApplicationError {
    Validity,
    Verification,
    // This error is returned also if the file-name is not set.
    Size,
    Compatibility,
    Other,
}

impl Display for UploadApplicationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UploadApplicationError::Validity => write!(f, "Not valid"),
            UploadApplicationError::Verification => write!(f, "Verification failed"),
            UploadApplicationError::Size => write!(f, "Too large"),
            UploadApplicationError::Compatibility => write!(f, "Not compatible"),
            UploadApplicationError::Other => write!(f, "Unspecified"),
        }
    }
}

impl Error for UploadApplicationError {}

impl FromStr for UploadApplicationError {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().strip_prefix("Error: ") {
            Some("1") => Ok(Self::Validity),
            Some("2") => Ok(Self::Verification),
            Some("3") => Ok(Self::Size),
            Some("5") => Ok(Self::Compatibility),
            Some("10") => Ok(Self::Other),
            Some(_) => Err("Unexpected code"),
            None => Err("Missing error prefix"),
        }
    }
}

#[derive(Debug)]
pub struct UploadRequest<'a> {
    client: &'a HttpClient,
    name: String,
    data: Vec<u8>,
}

impl<'a> UploadRequest<'a> {
    pub async fn send(self) -> Result<(), HttpRpcError<UploadApplicationError>> {
        let Self { client, name, data } = self;
        let response = client
            .post(PATH)
            .map_err(HttpRpcError::ParseUrl)?
            .replace_with(|b| b.multipart(Form::new().part("x", Part::bytes(data).file_name(name))))
            .send()
            .await
            .map_err(HttpRpcError::other)?;
        let status = response.status();
        let text = response.text().await.map_err(HttpRpcError::other)?;
        if text.trim() == "OK" {
            debug_assert_eq!(status, StatusCode::OK);
            return Ok(());
        }
        let e = text.parse().map_err(|e| {
            HttpRpcError::Other(anyhow::anyhow!(
                "Could not parse error {e} (status: {status}; text:{text}"
            ))
        })?;
        Err(HttpRpcError::Remote(e))
    }
}
pub struct Client<'a>(&'a HttpClient);

impl<'a> Client<'a> {
    pub fn new(http_client: &'a HttpClient) -> Self {
        Self(http_client)
    }

    // TODO: Consider returning a type that we control
    // Returning `io::Result` means the implementation cannot be changed to use fallible functions
    // the result of which cannot be mapped to `io::Error`.
    pub fn upload<P: AsRef<Path>>(&self, file: P) -> io::Result<UploadRequest> {
        let data = fs::read(&file)?;
        let name = file
            .as_ref()
            .file_name()
            .expect("read would have failed if path ended with ..")
            .to_string_lossy()
            .to_string();
        Ok(UploadRequest {
            client: self.0,
            name,
            data,
        })
    }
}
