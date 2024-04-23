//! A rust wrapper around the [License Key API] from [ACAP].
//!
//! This crate has two APIs:
//! * An [ergonomic API](`crate::ergo`) that is easy to use correctly, and
//! * A [flexible API](`crate::flex`) that closely follows the C API.
//!
//! The ergonomic API should be the default choice.
//!
//! [ACAP]: https://axiscommunications.github.io/acap-documentation/
//! [License Key API]: https://axiscommunications.github.io/acap-documentation/docs/api/native-sdk-api.html#license-key-api

pub use ergo::{verify, Error};

pub mod ergo;
pub mod flex;
