//! Bindings for the [Parameter management](https://www.axis.com/vapix-library/subjects/t10175981/section/t10036014/display).
// TODO: Return actionable errors.
// TODO: Implement remaining methods.
// TODO: Proper documentation.
// TODO: Consider encoding more knowledge about the API:
//  - Group hierarchies.
//  - The difference between dynamic and static groups.
//  - The effect of wildcards.
//  - Permissible parameter names.
// TODO: Consider adding control over the `usergroup` argument. This is unlikely to be needed by
//  apps but could be useful for tests.

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use anyhow::{bail, Context};
use reqwest::StatusCode;

pub const PATH: &str = "axis-cgi/param.cgi";

pub struct ListRequest {
    groups: Vec<String>,
}

impl ListRequest {
    /// Add another group to include in the list.
    ///
    /// If no groups are provided, all parameters will be returned when the request is executed.
    ///
    /// If any group is invalid, an error will be returned when the request is executed.
    ///
    /// # Panics
    ///
    /// Panics if group contains `,`.
    pub fn group(mut self, group: impl Display) -> Self {
        let group = group.to_string();
        // TODO: Consider removing asserts that don't shift errors left in a meaningful way.
        assert!(!group.contains(','));
        self.groups.push(group);
        self
    }

    pub async fn execute(
        self,
        client: &crate::http::Client,
    ) -> anyhow::Result<HashMap<String, String>> {
        let mut query = vec![("action", "list")];
        let group = self.groups.join(",");
        if !group.is_empty() {
            query.push(("group", &group));
        }
        let response = client
            .get(PATH)?
            .replace_with(|b| b.query(&query))
            .send()
            .await?;
        let status = response.status();
        let text = response
            .text()
            .await
            .with_context(|| format!("status code: {status}"))?;

        if let Some(e) = text.trim().strip_prefix("# Error: ") {
            bail!("{e}")
        }

        let mut untyped = HashMap::new();
        for line in text.lines() {
            let (k, v) = line
                .split_once('=')
                .with_context(|| format!("Expected at least one '=', but got {line:?}"))?;
            if untyped.insert(k.to_string(), v.to_string()).is_some() {
                bail!("Server sent key {k} more than once");
            }
        }
        Ok(untyped)
    }
}
// It would be nice to include information in these APIs about what type is returned, e.g. that
// `root.HTTPS.port` is no less than 1, no more than 65535, and can be encoded with a `u16`.
// It would also be nice to include information about what parameters can be expected to exist.
// But the `listdefinitions` action may be a better place for that since that response already
// includes that kind of metadata.
pub fn list() -> ListRequest {
    ListRequest { groups: Vec::new() }
}

pub struct UpdateRequest {
    parameters: HashMap<String, String>,
}

impl UpdateRequest {
    /// Add another parameter to be updated
    ///
    /// # Panics
    ///
    /// Panics if the same parameter is set twice or the `parameter` is one of the reserved words:
    /// - `action`
    /// - `usergroup`
    pub fn set<P: Debug + Display, V: Display>(self, parameter: P, value: V) -> Self {
        let parameter = parameter.to_string();
        assert_ne!(parameter, "action");
        assert_ne!(parameter, "usergroup");
        let mut parameters = self.parameters;
        assert_eq!(
            parameters.insert(parameter, value.to_string()),
            None,
            "Expected each parameter at most once"
        );
        Self { parameters }
    }

    pub async fn execute(self, client: &crate::http::Client) -> anyhow::Result<()> {
        let mut query = vec![("action", "update")];
        for (k, v) in &self.parameters {
            query.push((k, v));
        }
        let response = client
            .get(PATH)?
            .replace_with(|b| b.query(&query))
            .send()
            .await?;
        let status = response.status();
        let text = response
            .text()
            .await
            .with_context(|| format!("status code: {status}"))?;

        if status == StatusCode::OK {
            if text.trim() == "OK" {
                Ok(())
            } else {
                bail!("Expected {status} response to state OK but got {text}");
            }
        } else if let Some(e) = text.trim().strip_prefix("# Error: ") {
            bail!("{e}")
        } else {
            bail!("Expected {status} response to state an error, but got {text}")
        }
    }
}

pub fn update() -> UpdateRequest {
    UpdateRequest {
        parameters: HashMap::new(),
    }
}
