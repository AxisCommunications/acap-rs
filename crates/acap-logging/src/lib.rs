#![forbid(unsafe_code)]
#![allow(clippy::needless_doctest_main)]
#![doc = include_str!("../README.md")]
#[cfg(feature = "tty")]
use std::{env, io::IsTerminal};

use log::{debug, LevelFilter};

/// Builder used to customize the logging.
#[derive(Clone, Debug)]
pub struct Builder {
    level: LevelFilter,
    module_levels: Vec<(String, LevelFilter)>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            level: LevelFilter::Debug,
            module_levels: Vec::new(),
        }
    }
}

impl Builder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Same semantics as [`libsyslog::SyslogBuilder::level`].
    #[must_use]
    pub fn level(mut self, level: LevelFilter) -> Self {
        self.level = level;
        self
    }

    /// Same semantics as [`libsyslog::SyslogBuilder::module_level`].
    #[must_use]
    pub fn module_level(mut self, target: &str, level: LevelFilter) -> Self {
        self.module_levels.push((target.to_string(), level));
        self
    }

    #[cfg(feature = "tty")]
    fn directives_string(&self) -> String {
        let mut spec = self.level.as_str().to_string();
        for (target, level) in &self.module_levels {
            spec.push(',');
            spec.push_str(target);
            spec.push('=');
            spec.push_str(level.as_str());
        }
        spec
    }

    fn init_syslog(self) {
        let mut builder = libsyslog::Syslog::builder().level(self.level);
        for (target, level) in &self.module_levels {
            builder = builder.module_level(target, *level);
        }
        builder.build().init().unwrap();
    }

    /// Set up app-logging as appropriate for the environment:
    ///
    /// - If stdout is a terminal, write to stderr.
    /// - Otherwise, write to the system logger.
    ///
    /// # Panics
    ///
    /// This function will panic if
    /// it fails to initialize the appropriate logger or
    /// a global logger has already been initialized.
    pub fn init(self) {
        // Using `su -pc "..."` just says the "Connection to ... closed", and
        // I have not found another way to run as the SDK user over ssh and allocate a tty, so
        // if we detect an `env_logger` configuration, we write to stderr anyway.
        #[cfg(feature = "tty")]
        if std::io::stdout().is_terminal()
            || env::var_os("RUST_LOG").is_some()
            || env::var_os("RUST_LOG_STYLE").is_some()
        {
            env_logger::Builder::from_env(
                env_logger::Env::default().default_filter_or(self.directives_string()),
            )
            .init();
            debug!("Logging initialized");
            return;
        }

        self.init_syslog();
        debug!("Logging initialized");
    }
}

/// Set up app-logging as appropriate for the environment:
///
/// - If stdout is a terminal, write to stderr.
/// - Otherwise, write to the system logger.
///
/// # Panics
///
/// This function will panic if
/// it fails to initialize the appropriate logger or
/// a global logger has already been initialized.
pub fn init_logger() {
    Builder::new().init();
}
