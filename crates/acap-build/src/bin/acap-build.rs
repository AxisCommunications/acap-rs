//! A drop-in replacement for the acap-build python script
use acap_build::{AppBuilder, Architecture};
use anyhow::Context;
use clap::{Parser, ValueEnum};
use std::{
    env,
    fmt::{Display, Formatter},
    fs,
    path::PathBuf,
    process::Command,
};
use tempdir::TempDir;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
enum BuildOption {
    #[default]
    Make,
}

impl Display for BuildOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Make => write!(f, "make"),
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
    }

    let arch: Architecture = env::var("OECORE_TARGET_ARCH")?.parse()?;
    let manifest = match manifest {
        None => path.join("manifest.json"),
        Some(m) => path.join(m),
    };
    let license = path.join("LICENSE");

    let staging_dir = TempDir::new_in(&path, "acap-build")?;
    let mut builder = AppBuilder::new(true, staging_dir.path(), &manifest, arch)?;
    builder
        .add_exe(&path.join(builder.app_name()))?
        .add_license(&license)?;

    let lib = path.join("lib");
    if lib.exists() {
        builder.add_lib(&lib)?;
    }

    let html = path.join("html");
    if html.exists() {
        builder.add_html(&html)?;
    }

    for additional_file in additional_file {
        builder.add_additional(&path.join(additional_file))?;
    }

    let eap_file_name = builder.build()?;
    fs::copy(
        staging_dir.path().join(&eap_file_name),
        path.join(&eap_file_name),
    )?;

    Ok(())
}
