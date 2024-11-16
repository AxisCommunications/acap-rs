//! A drop-in replacement for the acap-build python script
use std::{
    env,
    fmt::{Display, Formatter},
    fs,
    fs::File,
    path::PathBuf,
    process::Command,
};

use acap_build::{AppBuilder, Architecture};
use clap::{Parser, ValueEnum};
use log::{debug, warn};
use tempdir::TempDir;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
enum BuildOption {
    #[default]
    Make,
    /// Note: this is experimental
    Meson,
    NoBuild,
}

impl Display for BuildOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Make => write!(f, "make"),
            Self::Meson => write!(f, "meson"),
            Self::NoBuild => write!(f, "no-build"),
        }
    }
}

#[derive(Clone, Debug, Parser)]
#[clap(verbatim_doc_comment)]
struct Cli {
    path: PathBuf,
    /// Build tool, if any, to run before packaging.
    #[clap(default_value_t, long, short)]
    build: BuildOption,
    // TODO: Look into mimicking the original interface exactly.
    /// Note: can be used more than once.
    #[clap(long)]
    meson_cross_files: Vec<PathBuf>,
    #[clap(long, short)]
    manifest: Option<PathBuf>,
    /// Note: can be used more than once.
    #[clap(long, short)]
    additional_file: Vec<PathBuf>,
    #[clap(long)]
    disable_manifest_validation: bool,
    #[clap(long)]
    disable_package_creation: bool,
}

impl Cli {
    fn exec(self) -> anyhow::Result<()> {
        let Self {
            path,
            build,
            meson_cross_files,
            manifest,
            additional_file,
            disable_manifest_validation,
            disable_package_creation,
        } = self;
        if !meson_cross_files.is_empty() {
            todo!()
        }
        if !disable_manifest_validation {
            warn!("Manifest validation is not implemented and will be skipped")
        }
        if disable_package_creation {
            todo!()
        }
        match build {
            BuildOption::Make => assert!(Command::new("make").status().unwrap().success()),
            BuildOption::Meson => todo!(),
            BuildOption::NoBuild => todo!(),
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
}

fn main() -> anyhow::Result<()> {
    let log_file = if env::var_os("RUST_LOG").is_none() {
        let dir = dirs::runtime_dir().unwrap_or(env::temp_dir());
        let path = dir.join("cargo-acap-sdk.log");
        let target = env_logger::Target::Pipe(Box::new(File::create(&path)?));
        let mut builder = env_logger::Builder::from_env(env_logger::Env::default());
        builder.target(target).filter_level(log::LevelFilter::Debug);
        builder.init();
        Some(path)
    } else {
        env_logger::init();
        None
    };
    debug!("Logging initialized");

    match Cli::parse().exec() {
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