#![doc = include_str!("../README.md")]
mod ajr;
mod ajr_http;
mod apis;
mod http;

pub use apis::{certificate_management, mqtt_client1, mqtt_event1, parameter_management};
pub use http::Client as HttpClient;

/// Expose some crate internals to make it easier for users to work around holes in the API.
///
/// <div class="warning">
///     This module will be removed eventually!
/// </div>
pub mod temporary {
    pub use super::ajr_http::execute_params;
}
