#![forbid(unsafe_code)]
#![allow(clippy::needless_doctest_main)]
#![doc = include_str!("../README.md")]
use std::{env, io::IsTerminal};

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
///
/// # Panics
///
/// This function will panic if
/// it fails to initialize the appropriate logger or
/// a global logger has already been initialized.
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
