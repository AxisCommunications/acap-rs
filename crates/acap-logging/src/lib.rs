#![forbid(unsafe_code)]
#![allow(clippy::needless_doctest_main)]
#![doc = include_str!("../README.md")]
#[cfg(feature = "tty")]
use std::{env, io::IsTerminal};

use log::{debug};

fn init_syslog() {
    libsyslog::Syslog::builder()
        .level(log::LevelFilter::Debug)
        .build()
        .init()
        .unwrap();
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
    // if we detect an `env_logger` configuration, we write to stderr anyway.
    #[cfg(feature = "tty")]
    if std::io::stdout().is_terminal()
        || env::var_os("RUST_LOG").is_some()
        || env::var_os("RUST_LOG_STYLE").is_some()
    {
        env_logger::init();
        debug!("Logging initialized");
        return;
    }
    
    init_syslog();
    debug!("Logging initialized");
}
