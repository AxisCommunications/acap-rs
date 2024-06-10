//! A simple hello world application
//!
//! Uses some common app-logging crates to demonstrate
//! 1. how to use them in an application, and
//! 2. how to build and bundle them as an application.

use log::info;
use std::collections::HashMap;

fn main() {
    app_logging::init_logger();
    info!("args: {:?}", std::env::args().collect::<Vec<_>>());
    info!("vars: {:#?}", std::env::vars().collect::<HashMap<_, _>>());
}
