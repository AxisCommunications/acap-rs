//! Bindings for the [Event API](https://axiscommunications.github.io/acap-documentation/docs/api/src/api/axevent/html/index.html).
//!
//! This crate provide two APIs with different goals:
//! - [`ergo`] strives to enable all but the most exotic use cases in an easy and idiomatic way.
//! - [`flex`] strives to facilitate transitioning from C.
pub mod ergo;
pub mod flex;
