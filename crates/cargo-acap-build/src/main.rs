#![forbid(unsafe_code)]
use std::fs::File;

use cargo_acap_build::{get_cargo_metadata, AppBuilder, Architecture};
use clap::{Parser, ValueEnum};
use log::debug;

// TODO: Figure out what to call this.
// This is sometimes called just "architecture" but in other contexts arch refers to the first
// part: https://clang.llvm.org/docs/CrossCompilation.html#target-triple
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, ValueEnum)]
enum ArchAbi {
    Aarch64,
    Armv7hf,
}

impl From<ArchAbi> for Architecture {
    fn from(val: ArchAbi) -> Self {
        match val {
            ArchAbi::Aarch64 => Architecture::Aarch64,
            ArchAbi::Armv7hf => Architecture::Armv7hf,
        }
    }
}

/// ACAP analog to `cargo build`.
#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// If given, build only for the given architecture(s).
    ///
    /// Can be used multiple times.
    #[arg(long)]
    target: Vec<ArchAbi>,
    /// Pass additional arguments to `cargo build`.
    ///
    /// Beware that not all incompatible arguments have been documented.
    args: Vec<String>,
}

impl Cli {
    pub fn targets(&self) -> Vec<Architecture> {
        if self.target.is_empty() {
            vec![Architecture::Aarch64, Architecture::Armv7hf]
        } else {
            self.target.iter().map(|&t| t.into()).collect()
        }
    }
}

fn build_and_copy(cli: Cli) -> anyhow::Result<()> {
    AppBuilder::from_targets(cli.targets())
        .args(cli.args)
        .artifact_dir(get_cargo_metadata()?.target_directory.join("acap"))
        .execute()?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let log_file = if std::env::var_os("RUST_LOG").is_none() {
        if let Some(runtime_dir) = dirs::runtime_dir() {
            let path = runtime_dir.join("cargo-acap-build.log");
            let target = env_logger::Target::Pipe(Box::new(File::create(&path)?));
            let mut builder = env_logger::Builder::from_env(env_logger::Env::default());
            builder.target(target).filter_level(log::LevelFilter::Debug);
            builder.init();
            Some(path)
        } else {
            None
        }
    } else {
        env_logger::init();
        None
    };
    debug!("Logging initialized");

    let cli = Cli::parse();

    match build_and_copy(cli) {
        Ok(()) => Ok(()),
        Err(e) => {
            if let Some(log_file) = log_file {
                Err(e.context(format!("A detailed log has been saved to {log_file:?}")))
            } else {
                Err(e)
            }
        }
    }
}
