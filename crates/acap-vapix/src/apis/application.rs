//! Bindings for the [Application API](https://www.axis.com/vapix-library/subjects/t10102231/section/t10036126/display).
// TODO: Return actionable errors.
// TODO: Implement remaining methods.
// TODO: Proper documentation.
// TODO: Consider encoding more knowledge about the API:

use std::fmt::{Debug, Display, Formatter};

use anyhow::{bail, Context};

pub const PATH: &str = "axis-cgi/applications/control.cgi";

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

        if let Some(e) = text.trim().strip_prefix("Error: ") {
            let code: u8 = e.parse().with_context(|| format!("Unexpeced code {e}"))?;
            match code {
                4 => bail!("not found (4)"),
                6 => bail!("already running (6)"),
                7 => bail!("not running (7)"),
                8 => bail!("could not start (8)"),
                9 => bail!("too many running application (9)"),
                10 => bail!("unspecified (10)"),
                _ => bail!("Unexpected error ({code})"),
            }
        }

        if text.trim() != "OK" {
            bail!("Expected OK, but got {text}")
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
