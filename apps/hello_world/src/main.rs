//! A simple hello world application
//!
//! Uses some common logging crates to demonstrate
//! 1. how to use them in an application, and
//! 2. how to build and bundle them as an application.

use log::info;
use std::collections::HashMap;

mod app_logging;

fn main() {
    app_logging::init_logger();
    info!("args: {:?}", std::env::args().collect::<Vec<_>>());
    info!("vars: {:#?}", std::env::vars().collect::<HashMap<_, _>>());
}
