use std::ops::Deref;

use anyhow::{bail, Context};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct CreateUser {
    username: String,
    password: String,
    comment: String,
}
#[derive(Serialize, Deserialize)]
struct UpdateUser {
    password: String,
}

#[derive(Serialize, Deserialize)]
struct RequestEnvelope<T> {
    data: T,
}

#[derive(Serialize, Deserialize)]
struct ResponseEnvelope {
    pub status: String,
    pub data: Option<String>,
}
pub async fn update_user(
    client: &acap_vapix::HttpClient,
    username: &str,
    password: &str,
) -> anyhow::Result<()> {
    let request_envelope = RequestEnvelope {
        data: UpdateUser {
            password: password.to_string(),
        },
    };
    let builder = client
        .put(&format!("config/rest/ssh/v1/users/{username}"))
        .unwrap()
        .replace_with(|b| b.json(&request_envelope));
    let resp = builder.send().await?;
    let status = resp.status();
    let text = resp.text().await?;
    let body: ResponseEnvelope = serde_json::from_str(&text).context(text.to_string())?;
    if (status, body.status.deref()) != (StatusCode::OK, "success") {
        bail!("Server returned error: {status} {text:?}")
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_serialize_update_request() {
        let req = RequestEnvelope {
            data: UpdateUser {
                password: "pass".to_string(),
            },
        };
        assert_eq!(
            serde_json::to_string(&req).unwrap(),
            r#"{"data":{"password":"pass"}}"#
        )
    }

    #[test]
    fn can_deserialize_update_response() {
        let res = r#"{
            "status": "success"
        }"#;
        let _: ResponseEnvelope = serde_json::from_str(res).unwrap();
    }
}
