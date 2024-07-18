use anyhow::{bail, Context};
use std::fs;
use std::fs::File;
use std::path::PathBuf;

use cargo_acap_build::{build, get_cargo_metadata, Architecture};
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

fn copy_eaps(artifacts: Vec<PathBuf>) -> anyhow::Result<()> {
    let cargo_target_dir = get_cargo_metadata()?.target_directory;
    let acap_dir = cargo_target_dir.join("acap");
    match fs::create_dir(&acap_dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => Err(e)?,
    }
    // Note that:
    // - The app name must contain no hyphens and by convention we give the package the same name.
    // - Test binaries are usually named like `{package_name}`-{hex_string}`
    // This means we should be able to guess the app name from the package name and use the reverse
    // of that to give `.eap` files unique names.
    // TODO: Consider exposing this from lib instead of guessing it here
    for src in artifacts {
        if let Some(extension) = src.extension() {
            if extension.to_string_lossy() != "eap" {
                debug!("{src:?} is not an `.eap`");
                continue;
            }
        }
        let to = src
            .parent()
            .context(".eap file has no parent dir")?
            .file_name()
            .context("dir has no name")?
            .to_str()
            .context("dir name is not a valid string")?;
        let parts: Vec<_> = to.split('-').collect();
        let from = match parts.len() {
            0 => panic!("Every string splits into at least one substring"),
            1 | 2 => parts[0],
            _ => bail!("Expected dir name with at most one '-' but got {to:?}"),
        };
        let name = src
            .file_name()
            .context("eap has no file name")?
            .to_str()
            .context("eap file name is not a valid string")?
            .replace(from, to);
        let dst = acap_dir.join(name);
        debug!("Copying `.eap` from {src:?} to {dst:?}");
        fs::copy(src, dst)?;
    }
    Ok(())
}

fn build_and_copy(cli: Cli) -> anyhow::Result<()> {
    let targets = cli.targets();
    let args: Vec<_> = cli.args.iter().map(|s| s.as_str()).collect();
    let artifacts = build(&targets, &args)?;
    copy_eaps(artifacts)
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
