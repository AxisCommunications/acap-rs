//! Support for implementing bindings that use Axis JSON RPC (AJR), regardless of transport.
//!
//! This module is independent of how the RPCs are transported. Transport specific utilities are
//! provided by separate modules:
//! - [`crate::ajr_http`]
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

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
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

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};

    use super::*;

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

#[cfg(test)]
mod test_implementation_alternatives {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    use crate::ajr::Error;

    const GOOD_DATA: &str = r#"{"apiVersion":"1.0","method":"myMethod","data":{"camelCase":127}}"#;
    const BAD_DATA: &str = r#"{"apiVersion":"1.0","method":"myMethod","data":{"camelCase":128}}"#;

    #[test]
    fn explore_generic_enum_data() {
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
            Error(TaggedError),
            Data(T),
        }

        #[derive(Debug, Deserialize, Serialize)]
        struct TaggedError {
            #[serde(skip_serializing_if = "Option::is_none")]
            method: Option<String>,
            error: Error,
        }

        #[derive(Debug, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase", tag = "method", content = "data")]
        enum Data {
            MyMethod(MyMethodData),
        }

        // Cons: Since the property name is not the conventional `snake_case`, additional
        // boilerplate is needed to handle cases like this.
        #[derive(Debug, Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        struct MyMethodData {
            camel_case: i8,
        }

        let _ = serde_json::from_str::<ResponseEnvelope<Data>>(GOOD_DATA)
            .unwrap()
            .data()
            .unwrap();
        // Cons: Since an untagged enum is used this error message is worse unhelpful.
        assert_eq!(
            serde_json::from_str::<ResponseEnvelope<Data>>(BAD_DATA)
                .err()
                .unwrap()
                .to_string(),
            "data did not match any variant of untagged enum EnvelopeResult at line 1 column 65"
        );
    }
    #[test]
    fn explore_generic_struct_data() {
        // Con: If the transport does not handle pairing of request and response,
        // then this cannot be used as is.
        #[derive(Debug, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ResponseEnvelope<T> {
            api_version: String,
            method: Option<String>,
            #[serde(flatten)]
            result: TaggedResult<T>,
        }

        impl<T> ResponseEnvelope<T> {
            pub fn data(self) -> Result<T, Error> {
                match self.result {
                    TaggedResult::Error(error) => Err(error),
                    TaggedResult::Data(d) => Ok(d),
                }
            }
        }

        #[derive(Debug, Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        enum TaggedResult<T> {
            Data(T),
            Error(Error),
        }

        #[derive(Debug, Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        struct MyMethodData {
            camel_case: i8,
        }

        let _ = serde_json::from_str::<ResponseEnvelope<MyMethodData>>(GOOD_DATA)
            .unwrap()
            .data()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<ResponseEnvelope<MyMethodData>>(BAD_DATA)
                .err()
                .unwrap()
                .to_string(),
            "invalid value: integer `128`, expected i8 at line 1 column 65"
        );
    }

    #[test]
    /// In this implementation the parsing of the data is done in a separate step.
    ///
    fn explore_dynamic_struct_data() {
        #[derive(Debug, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ResponseEnvelope {
            api_version: String,
            method: Option<String>,
            #[serde(flatten)]
            result: TaggedResult,
        }

        // Pro: Since the parsing is done in two steps, it is possible to provide more precise error
        //  messages when it fails; it is more common for deserialization of the data to fail
        //  (because it is too strict) than deserialization of the envelope.
        // Con: The `ajr` module needs to be aware of, and able to communicate, deserialization
        //  errors. This blurs the line between transport errors and RPC errors further.
        impl ResponseEnvelope {
            pub fn data<T: for<'a> Deserialize<'a>>(self) -> anyhow::Result<T> {
                match self.result {
                    TaggedResult::Error(e) => Err(e.into()),
                    TaggedResult::Data(d) => Ok(serde_json::from_value::<T>(d)?),
                }
            }
        }

        #[derive(Debug, Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        enum TaggedResult {
            // It may be possible to improve performance by using `RawValue` instead of `Value`.
            Data(Value),
            Error(Error),
        }

        #[derive(Debug, Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        struct MyMethodData {
            camel_case: i8,
        }

        let _ = serde_json::from_str::<ResponseEnvelope>(GOOD_DATA)
            .unwrap()
            .data::<MyMethodData>()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<ResponseEnvelope>(BAD_DATA)
                .unwrap()
                .data::<MyMethodData>()
                .err()
                .unwrap()
                .to_string(),
            "invalid value: integer `128`, expected i8"
        );
    }
}
