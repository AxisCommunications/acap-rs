//! Utilities for managing app-logging in an application.

use std::env;
use std::io::IsTerminal;

use log::debug;

fn init_syslog() {
    let formatter = syslog::Formatter3164::default();
    let logger = syslog::unix(formatter).unwrap();
    log::set_boxed_logger(Box::new(syslog::BasicLogger::new(logger))).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
}

/// Set up app-logging as appropriate for the environment, then run the provided function.
///
/// If stdout is a terminal, write to stderr.
/// Otherwise, write to the system logger.
pub fn init_logger() {
    // Using `su -pc "..."` just says the "Connection to ... closed", and
    // I have not found another way to run as the SDK user over ssh and allocate a tty, so
    // if we detect an `env_logger` configuration we write to stderr anyway.
    if std::io::stdout().is_terminal()
        || env::var_os("RUST_LOG").is_some()
        || env::var_os("RUST_LOG_STYLE").is_some()
    {
        env_logger::init();
    } else {
        init_syslog();
    }
    debug!("Logging initialized");
}
