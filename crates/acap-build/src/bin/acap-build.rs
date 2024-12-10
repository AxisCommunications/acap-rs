//! A drop-in replacement for the acap-build python script
use std::{
    env,
    fmt::{Display, Formatter},
    fs,
    path::PathBuf,
    process::Command,
};

use acap_build::{AppBuilder, Architecture};
use anyhow::Context;
use clap::{Parser, ValueEnum};
use log::debug;
use tempdir::TempDir;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
enum BuildOption {
    #[default]
    Make,
    NoBuild,
}

impl Display for BuildOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Make => write!(f, "make"),
            Self::NoBuild => write!(f, "no-build"),
        }
    }
}

#[derive(Clone, Debug, Parser)]
struct Cli {
    path: PathBuf,
    /// Build tool, if any, to run before packaging.
    #[clap(default_value_t, long, short)]
    build: BuildOption,
    #[clap(long, short)]
    manifest: Option<PathBuf>,
    /// Note: can be used more than once.
    #[clap(long, short)]
    additional_file: Vec<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let Cli {
        path,
        build,
        manifest,
        additional_file,
    } = Cli::parse();
    match build {
        BuildOption::Make => assert!(Command::new("make")
            .status()
            .context("subprocess make failed")?
            .success()),
        BuildOption::NoBuild => {
            debug!("no build");
        }
    }

    let arch: Architecture = env::var("OECORE_TARGET_ARCH")?.parse()?;
    let manifest = match manifest {
        None => path.join("manifest.json"),
        Some(m) => path.join(m),
    };

    let staging_dir = TempDir::new_in(&path, "acap-build")?;
    let mut builder = AppBuilder::new(true, staging_dir.path(), &manifest, arch)?;

    for name in builder.mandatory_files() {
        builder.add(&path.join(name))?;
    }

    for name in builder.optional_files() {
        let file = path.join(name);
        if file.symlink_metadata().is_ok() {
            builder.add(&file)?;
        }
    }

    for additional_file in additional_file {
        builder.add(&path.join(additional_file))?;
    }

    let eap_file_name = builder.build()?;
    fs::copy(
        staging_dir.path().join(&eap_file_name),
        path.join(&eap_file_name),
    )?;

    Ok(())
}
