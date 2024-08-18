//! Bindings for the [Application API](https://www.axis.com/vapix-library/subjects/t10102231/section/t10036126/display).
// TODO: Return more actionable errors.
// TODO: Implement remaining methods.
// TODO: Proper documentation.

use std::fmt::{Debug, Display, Formatter};

use anyhow::Context;

pub const PATH: &str = "axis-cgi/applications/control.cgi";

#[derive(Debug)]
struct ControlError(u8);

impl Display for ControlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            4 => write!(f, "not found (4)"),
            6 => write!(f, "already running (6)"),
            7 => write!(f, "not running (7)"),
            8 => write!(f, "could not start (8)"),
            9 => write!(f, "too many running application (9)"),
            10 => write!(f, "other (10)"),
            _ => unreachable!(),
        }
    }
}

impl TryFrom<u8> for ControlError {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            4 | 6 | 7 | 8 | 9 | 10 => Ok(Self(value)),
            _ => Err(anyhow::anyhow!("Unexpected code {value}")),
        }
    }
}

#[derive(Debug)]
enum InnerError {
    Control(ControlError),
    Other(anyhow::Error),
}

#[derive(Debug)]
pub struct Error(InnerError);

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            InnerError::Control(e) => Display::fmt(e, f),
            InnerError::Other(e) => Display::fmt(e, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let InnerError::Other(e) = &self.0 {
            e.source()
        } else {
            None
        }
    }
}

impl Error {
    fn from_code(c: u8) -> anyhow::Result<Self> {
        Ok(Self(InnerError::Control(c.try_into()?)))
    }
    fn from_other<E: Into<anyhow::Error>>(e: E) -> Self {
        Self(InnerError::Other(e.into()))
    }
}

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

    pub async fn execute(self, client: &crate::http::Client) -> Result<(), Error> {
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
            .post(PATH)
            .map_err(Error::from_other)?
            .replace_with(|b| b.query(&query))
            .send()
            .await
            .map_err(Error::from_other)?;
        let status = response.status();
        let text = response
            .text()
            .await
            .with_context(|| format!("status code: {status}"))
            .map_err(Error::from_other)?;

        if let Some(e) = text.trim().strip_prefix("Error: ") {
            let code: u8 = e
                .parse()
                .with_context(|| format!("Unexpected code {e}"))
                .map_err(Error::from_other)?;
            return Err(Error::from_code(code).map_err(Error::from_other)?);
        }

        if text.trim() != "OK" {
            return Err(Error::from_other(anyhow::anyhow!(
                "Expected OK, but got {text}"
            )));
        }

        Ok(())
    }
}

pub fn control<T: Display>(action: Action, package: T) -> ControlRequest {
    ControlRequest {
        action,
        package: package.to_string(),
        returnpage: None,
    }
}
