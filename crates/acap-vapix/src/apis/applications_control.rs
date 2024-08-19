//! Bindings for [part of the Application API](https://www.axis.com/vapix-library/subjects/t10102231/section/t10036126/display).
// TODO: Return more actionable errors.
// TODO: Adopt consistent error reporting strategy.
// TODO: Proper documentation.

use std::fmt::{Debug, Display, Formatter};

use anyhow::{anyhow, Context};
use reqwest::StatusCode;

pub const PATH: &str = "axis-cgi/applications/control.cgi";

#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    NotFound,
    AlreadyRunning,
    NotRunning,
    CouldNotSTart,
    TooManyRunning,
    Other,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "not found (4)"),
            Self::AlreadyRunning => write!(f, "already running (6)"),
            Self::NotRunning => write!(f, "not running (7)"),
            Self::CouldNotSTart => write!(f, "could not start (8)"),
            Self::TooManyRunning => write!(f, "too many running application (9)"),
            Self::Other => write!(f, "other (10)"),
        }
    }
}

impl std::error::Error for Error {}

#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Start,
    Stop,
    Restart,
    Remove,
}

impl Display for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Start => write!(f, "start"),
            Action::Stop => write!(f, "stop"),
            Action::Restart => write!(f, "restart"),
            Action::Remove => write!(f, "remove"),
        }
    }
}

pub struct ControlRequest {
    action: Action,
    package: String,
    returnpage: Option<String>,
}

impl ControlRequest {
    pub fn returnpage(mut self, returnpage: impl Display) -> Self {
        self.returnpage = Some(returnpage.to_string());
        self
    }

    pub async fn execute(self, client: &crate::http::Client) -> anyhow::Result<()> {
        let Self {
            action,
            package,
            returnpage,
        } = self;
        let action = action.to_string();
        let mut query = vec![("action", &action), ("package", &package)];
        if let Some(returnpage) = &returnpage {
            query.push(("returnpage", returnpage));
        }

        let response = client
            .post(PATH)?
            .replace_with(|b| b.query(&query))
            .send()
            .await?;

        let status = response.status();
        let text = response
            .text()
            .await
            .with_context(|| format!("status code: {status}"))?;

        if text.trim() == "OK" {
            debug_assert_eq!(status, StatusCode::OK);
            return Ok(());
        }

        let e = match text.trim().strip_prefix("Error: ") {
            Some("4") => Error::NotFound.into(),
            Some("6") => Error::AlreadyRunning.into(),
            Some("7") => Error::NotRunning.into(),
            Some("8") => Error::CouldNotSTart.into(),
            Some("9") => Error::TooManyRunning.into(),
            Some("10") => Error::Other.into(),
            Some(_) | None => anyhow!("Unexpected response").context(status).context(text),
        };
        Err(e)
    }
}

pub fn control<T: Display>(action: Action, package: T) -> ControlRequest {
    ControlRequest {
        action,
        package: package.to_string(),
        returnpage: None,
    }
}
