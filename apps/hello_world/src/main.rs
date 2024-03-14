//! A simple hello world application
//!
//! Uses some common logging crates to demonstrate
//! 1. how to use them in an application, and
//! 2. how to build and bundle them as an application.

use log::info;

mod app_logging;

fn main() {
    app_logging::init_logger();
    info!("Hello World!");
}
