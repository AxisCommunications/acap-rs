//! Bindings for the [Parameter management](https://www.axis.com/vapix-library/subjects/t10175981/section/t10036014/display).
use std::{collections::HashMap, fmt::Debug};

use anyhow::{bail, Context};

pub struct ListRequest {
    groups: Vec<String>,
}

impl ListRequest {
    /// Add another group to include in the list.
    ///
    /// If no groups are provided, all parameters will be returned.
    ///
    /// If any group is invalid, an error will be returned.
    pub fn group(self, group: impl ToString) -> Self {
        let mut groups = self.groups;
        groups.push(group.to_string());
        Self { groups }
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
            .get("/axis-cgi/param.cgi")?
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
            untyped.insert(k.to_string(), v.to_string());
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
    /// Panics if the same parameter is set twice or the parameter name is "action".
    pub fn set<P: Debug + ToString, V: ToString>(self, parameter: P, value: V) -> Self {
        let parameter = parameter.to_string();
        assert_ne!(parameter, "action");
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
        // query.extend(self.parameters.iter());
        let response = client
            .get("/axis-cgi/param.cgi")?
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
        if text.trim() != "OK" {
            bail!("Expected error or ok but got {text}")
        }
        Ok(())
    }
}

pub fn update() -> UpdateRequest {
    UpdateRequest {
        parameters: HashMap::new(),
    }
}
