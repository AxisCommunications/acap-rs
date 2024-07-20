/// Reusable functionality clients and servers that use some dialect of Axis JSON RPC (AJR).
// TODO: Consider handcrafting [de]serialization.
// This should allow us to:
// * simplify types
// * provide better error when deserialization fails
// * fail or warn when messages don't strictly conform to the standard
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestEnvelope<P> {
    api_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
    #[serde(flatten)]
    params: P,
}

impl<P> RequestEnvelope<P> {
    pub(crate) fn new(api_version: impl ToString, context: Option<String>, params: P) -> Self {
        Self {
            api_version: api_version.to_string(),
            context: context.map(|c| c.to_string()),
            params,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseEnvelope<T> {
    api_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
    #[serde(flatten)]
    result: EnvelopeResult<T>,
}

impl<T> ResponseEnvelope<T> {
    #[cfg(test)]
    pub fn new_data(api_version: impl ToString, context: Option<String>, data: T) -> Self {
        Self {
            api_version: api_version.to_string(),
            context: context.map(|c| c.to_string()),
            result: EnvelopeResult::Data(data),
        }
    }
    #[cfg(test)]
    fn new_error(api_version: impl ToString, context: Option<String>, error: Error) -> Self {
        Self {
            api_version: api_version.to_string(),
            context: context.map(|c| c.to_string()),
            result: EnvelopeResult::Error(TaggedError {
                method: None,
                error,
            }),
        }
    }
    pub fn data(self) -> Result<T, Error> {
        match self.result {
            EnvelopeResult::Error(TaggedError { error, .. }) => Err(error),
            EnvelopeResult::Data(d) => Ok(d),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
enum EnvelopeResult<T> {
    // If a response contains both a valid data and error, it will be silently deserialized to one.
    // List error first, so that this variant is preferred.
    Error(TaggedError),
    Data(T),
}

#[derive(Debug, Deserialize, Serialize)]
struct TaggedError {
    // We need to be able to capture the method but since all methods return the same error type
    // there's no enum tag that can capture this for us.
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<String>,
    error: Error,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, thiserror::Error)]
pub struct Error {
    code: u32,
    message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self { code, message } = self;
        write!(f, "{message} ({code})")
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};

    use super::*;

    // Pros of wrapping requests in enum:
    // * Serde can be used to communicate the method instead of a separate trait.
    //   Since most APIs support only HTTP the method could also be passed as an argument,
    //   but either way this is a bit of a nuisance.
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase", tag = "method", content = "params")]
    enum Params {
        Empty,
        EmptyNamed {},
        // `EmptyUnnamed()`  would be serialized as a list, so don't do it
        Named { foo: u32 },
        Unnamed(Bar),
    }

    #[derive(Clone, Eq, Debug, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase", tag = "method", content = "data")]
    enum Data {
        Named { foo: u32 },
        Unnamed(Bar),
        Empty,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Bar {
        bar: u32,
    }

    #[test]
    fn can_serialize_and_deserialize_request_examples() {
        let before = Params::Empty;
        let request = RequestEnvelope::new(1, None, before.clone());
        let text = serde_json::to_string(&request).unwrap();
        let after = serde_json::from_str::<RequestEnvelope<Params>>(&text)
            .unwrap()
            .params;
        assert_eq!(text, r#"{"apiVersion":"1","method":"empty"}"#);
        assert_eq!(before, after);

        let before = Params::EmptyNamed {};
        let request = RequestEnvelope::new(1, None, before.clone());
        let text = serde_json::to_string(&request).unwrap();
        let after = serde_json::from_str::<RequestEnvelope<Params>>(&text)
            .unwrap()
            .params;
        assert_eq!(
            text,
            r#"{"apiVersion":"1","method":"emptyNamed","params":{}}"#
        );
        assert_eq!(before, after);

        let before = Params::Named { foo: 123 };
        let request = RequestEnvelope::new(1, None, before.clone());
        let text = serde_json::to_string(&request).unwrap();
        let after = serde_json::from_str::<RequestEnvelope<Params>>(&text)
            .unwrap()
            .params;
        assert_eq!(
            text,
            r#"{"apiVersion":"1","method":"named","params":{"foo":123}}"#
        );
        assert_eq!(before, after);

        let before = Params::Unnamed(Bar { bar: 234 });
        let deserialized = RequestEnvelope::new(1, None, before.clone());
        let serialized = serde_json::to_string(&deserialized).unwrap();
        let after = serde_json::from_str::<RequestEnvelope<Params>>(&serialized)
            .unwrap()
            .params;
        assert_eq!(
            serialized,
            r#"{"apiVersion":"1","method":"unnamed","params":{"bar":234}}"#
        );
        assert_eq!(before, after);
    }
    #[test]
    fn can_serialize_and_deserialize_data_response_examples() {
        // TODO: Consider asserting that the serialized representation is as expected
        for before in [
            Data::Named { foo: 123 },
            Data::Unnamed(Bar { bar: 234 }),
            Data::Empty,
        ] {
            let deserialized = ResponseEnvelope::new_data(1, None, before.clone());
            let serialized = serde_json::to_string(&deserialized).unwrap();
            let after = serde_json::from_str::<ResponseEnvelope<Data>>(&serialized)
                .unwrap()
                .data()
                .unwrap();
            assert_eq!(before, after);
        }
    }
    #[test]
    fn can_serialize_and_deserialize_error_response() {
        // TODO: Consider asserting that the serialized representation is as expected
        let before = Error {
            code: 123,
            message: "Oops".to_string(),
        };
        let text = serde_json::to_string(&ResponseEnvelope::<Data>::new_error(
            1,
            None,
            before.clone(),
        ))
        .unwrap();
        println!("{text}");
        let after = serde_json::from_str::<ResponseEnvelope<Data>>(&text)
            .unwrap()
            .data()
            .err()
            .unwrap();
        assert_eq!(before, after)
    }
    #[test]
    fn can_deserialize_and_serialize_good_responses() {
        // TODO: Consider asserting that the deserialized representation is as expected
        let texts = vec![
            r#"{"apiVersion":"0","error":{"code":1000,"message":"Oops"}}"#,
            r#"{"apiVersion":"0","method":"named","error":{"code":1000,"message":"Oops"}}"#,
            r#"{"apiVersion":"0","method":"named","data":{"foo":1}}"#,
            r#"{"apiVersion":"0","method":"unnamed","data":{"bar":1}}"#,
            r#"{"apiVersion":"0","method":"empty"}"#,
        ];
        for expected in texts {
            let deserialized: ResponseEnvelope<Data> = serde_json::from_str(expected).unwrap();
            println!("{deserialized:?}");
            let actual = serde_json::to_string(&deserialized).unwrap();
            assert_eq!(actual, expected);
        }
    }
    #[test]
    fn cannot_deserialize_bad_responses() {
        let texts = vec![
            // Wrong key in data
            r#"{"apiVersion":"0","method":"named","data":{"bar":2}}"#,
            // Missing data (content)
            r#"{"apiVersion":"0","method":"named"}"#,
            // Missing data (method)
            r#"{"apiVersion":"0","data":{"foo":1}}"#,
            // Neither data nor error
            r#"{"apiVersion":"0"}"#,
        ];
        for expected in texts {
            assert!(serde_json::from_str::<ResponseEnvelope<Data>>(expected).is_err());
        }
    }

    #[test]
    fn can_deserialize_weird_responses() {
        // TODO: Consider asserting that the deserialized representation is as expected

        // Extra key in data
        let before = r#"{"apiVersion":"0","method":"named","data":{"foo":1,"bar":2}}"#;
        let deserialized: ResponseEnvelope<Data> = serde_json::from_str(before).unwrap();
        let after = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(
            after,
            r#"{"apiVersion":"0","method":"named","data":{"foo":1}}"#
        );

        // Both data and error
        let before = r#"{"apiVersion":"0","method":"named","data":{"foo":1},"error":{"code":1000,"message":"Oops"}}"#;
        let deserialized: ResponseEnvelope<Data> = serde_json::from_str(before).unwrap();
        let after = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(
            after,
            r#"{"apiVersion":"0","method":"named","error":{"code":1000,"message":"Oops"}}"#
        );
    }

    #[test]
    fn can_deserialize_arbitrary_data() {
        let serialized =
            r#"{"apiVersion":"0","method":"named","data":{"foo":1,"bar":{"foobar":2}}}"#;
        let deserialized: ResponseEnvelope<Value> = serde_json::from_str(serialized).unwrap();
        let _actual = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(
            deserialized.data().unwrap(),
            json!({"data":{"foo":1,"bar":{"foobar":2}},"method":"named"})
        );
        // We don't expect that serializing will give the original text back because `Value`
        // does not retain order.
    }

    #[test]
    fn supports_untagged_response() {
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Foo {
            foo: u32,
        }

        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase", untagged)]
        enum UntaggedData {
            Named { data: Foo },
        }

        println!(
            "{}",
            serde_json::to_string(&ResponseEnvelope::new_data(
                0,
                None,
                UntaggedData::Named {
                    data: Foo { foo: 1 }
                }
            ))
            .unwrap()
        );

        let before = r#"{"apiVersion":"0","data":{"foo":1}}"#;
        let deserialized: ResponseEnvelope<UntaggedData> = serde_json::from_str(before).unwrap();
        let after = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(before, after);
    }
}
