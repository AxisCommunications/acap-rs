use std::fmt::{Display, Formatter};

use anyhow::{bail, Context};
use regex::RegexBuilder;
use reqwest::StatusCode;

macro_rules! map {
    ($($k:expr => $v:expr),* $(,)?) => {{
        std::collections::HashMap::from([$(($k, $v),)*])
    }};
}

fn extract_body(html: &str) -> Option<&str> {
    // Unwrapping is OK because the regex is hardcoded.
    let re = RegexBuilder::new(r"<body.*?>(.*?)</body>")
        .dot_matches_new_line(true)
        .build()
        .unwrap();
    // Unwrapping is OK because we know that the regex has a capture group.
    Some(re.captures(html)?.get(1).unwrap().as_str())
}

#[derive(Clone, Copy, Debug)]
pub enum Role {
    Viewer,
    OperatorViewer,
    AdminOperatorViewerPtz,
}
impl Display for Role {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Viewer => write!(f, "viewer"),
            Role::OperatorViewer => write!(f, "operator:viewer"),
            Role::AdminOperatorViewerPtz => write!(f, "admin:operator:viewer:ptz"),
        }
    }
}
pub enum Group {
    Root,
    Users,
}

impl Display for Group {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Group::Root => write!(f, "root"),
            Group::Users => write!(f, "users"),
        }
    }
}

const PATH: &str = "axis-cgi/pwdgrp.cgi";
// TODO: Improve generality
pub async fn add(
    client: &acap_vapix::HttpClient,
    username: &str,
    password: &str,
    group: Group,
    strict: bool,
    role: Role,
) -> anyhow::Result<()> {
    let role = role.to_string();
    let group = group.to_string();
    let mut query = map!(
        "action" => "add",
        "user" => username,
        "pwd" => password,
        "grp" => &group,
        "sgrp" => &role,
    );
    if strict {
        query.insert("strict_pwd", "1");
    }
    let resp = client
        .get(PATH)
        .unwrap()
        .replace_with(|b| b.query(&query))
        .send()
        .await?
        .error_for_status()?;
    let status = resp.status();
    let text = resp.text().await?;
    let body = extract_body(&text).context(text.clone())?;
    if (status, body.trim()) != (StatusCode::OK, &format!("Created account {username}.")) {
        bail!("Unexpected status and/or body: {status} {body:?}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn can_extract_body() {
        assert_eq!(
            super::extract_body(include_str!("pwdgrp/add_10_12_initial_response.html"))
                .unwrap()
                .trim(),
            "Created account root."
        );
    }
}
