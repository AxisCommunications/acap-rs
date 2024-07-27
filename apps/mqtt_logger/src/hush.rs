use std::fmt::{Debug, Display};

use serde::{Deserialize, Serialize};

// TODO: Consider not implementing deserialize
#[derive(Clone, Deserialize, Serialize)]
pub struct Secret(String);

impl Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Secret(***)")
    }
}

impl<T> From<T> for Secret
where
    T: Display,
{
    fn from(s: T) -> Self {
        Self(s.to_string())
    }
}

impl Secret {
    pub fn revealed(&self) -> &str {
        &self.0
    }
}
