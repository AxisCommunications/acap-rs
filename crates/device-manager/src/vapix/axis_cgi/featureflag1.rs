use std::collections::HashMap;

use acap_vapix::HttpClient;
use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::vapix::ajr_http;

const PATH: &str = "axis-cgi/featureflag.cgi";
const VERSION: &str = "1.0";
// TODO: Make builder the primary interface
// To enable a consistent API we probably want the user to first create a builder, then call it with
// a client.
pub async fn set(client: &HttpClient, flag_values: HashMap<String, bool>) -> anyhow::Result<()> {
    let data = ajr_http::exec(
        client,
        PATH,
        VERSION,
        None,
        Params::Set(SetParams { flag_values }),
    )
    .await?;
    let Data::Set { result } = data else {
        bail!("Expected Data::Set but got {data:?}");
    };
    if result != "Success" {
        bail!(r#"Expected result "Success" but got {result}"#);
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "method", content = "params")]
enum Params {
    ListAll,
    Set(SetParams),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetParams {
    flag_values: HashMap<String, bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "method", content = "data")]
enum Data {
    Set { result: String },
    ListAll { flags: Vec<Flag> },
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Flag {
    pub name: String,
    pub value: bool,
    description: String,
    default_value: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vapix::ajr::{RequestEnvelope, ResponseEnvelope};

    macro_rules! to_string_map {
    ($($k:expr => $v:expr),* $(,)?) => {{
        std::collections::HashMap::from([$(($k.to_string(), $v),)*])
    }};
}

    #[test]
    fn can_serialize_request() {
        let req = RequestEnvelope::new(
            VERSION,
            None,
            Params::Set(SetParams {
                flag_values: to_string_map! {"restrictRootAccess" => false},
            }),
        );
        assert_eq!(
            serde_json::to_string(&req).unwrap(),
            r#"{"apiVersion":"1.0","method":"set","params":{"flagValues":{"restrictRootAccess":false}}}"#
        )
    }

    #[test]
    fn can_deserialize_response() {
        let res = r#"{"apiVersion":"1.0","method":"set","data":{"result":"Success"}}"#;
        let _: ResponseEnvelope<Data> = serde_json::from_str(res).unwrap();
    }
}
