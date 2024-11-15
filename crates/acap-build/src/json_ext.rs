//! Additional methods for [`serde_json`] types.
use std::{
    any::type_name_of_val,
    fmt::{Debug, Display, Formatter},
};

use serde_json::{Map, Value};

#[derive(Debug)]
pub(crate) enum Error {
    KeyNotFound(&'static str),
    WrongType {
        key: &'static str,
        expected: &'static str,
        found: &'static str,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::KeyNotFound(key) => {
                write!(f, "Expected key {key}")
            }
            Error::WrongType {
                key,
                expected,
                found,
            } => {
                write!(f, "Expected {key} to be {expected}, but found {found}")
            }
        }
    }
}

impl std::error::Error for Error {}

pub(crate) type Result<T> = core::result::Result<T, Error>;
pub(crate) trait MapExt {
    fn to_object(&self) -> &Map<String, Value>;

    fn try_get_array(&self, key: &'static str) -> Result<&Vec<Value>> {
        match self.to_object().get(key) {
            Some(Value::Array(a)) => Ok(a),
            Some(v) => Err(Error::WrongType {
                key,
                expected: "array",
                found: type_name_of_val(v),
            }),
            None => Err(Error::KeyNotFound(key)),
        }
    }

    fn try_get_object(&self, key: &'static str) -> Result<&Map<String, Value>> {
        match self.to_object().get(key) {
            Some(Value::Object(o)) => Ok(o),
            Some(v) => Err(Error::WrongType {
                key,
                expected: "object",
                found: type_name_of_val(v),
            }),
            None => Err(Error::KeyNotFound(key)),
        }
    }

    fn try_get_str(&self, key: &'static str) -> Result<&str> {
        match self.to_object().get(key) {
            Some(Value::String(s)) => Ok(s),
            Some(v) => Err(Error::WrongType {
                key,
                expected: "string",
                found: type_name_of_val(v),
            }),
            None => Err(Error::KeyNotFound(key)),
        }
    }
}

impl MapExt for Map<String, Value> {
    fn to_object(&self) -> &Map<String, Value> {
        self
    }
}

pub(crate) trait ValueExt {
    fn try_to_object(&self) -> Result<&Map<String, Value>>;
}

impl ValueExt for Value {
    fn try_to_object(&self) -> Result<&Map<String, Value>> {
        match self {
            Value::Object(o) => Ok(o),
            v => Err(Error::WrongType {
                key: "?",
                expected: "object",
                found: type_name_of_val(v),
            }),
        }
    }
}
