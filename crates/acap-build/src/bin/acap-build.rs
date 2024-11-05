//! Wrapper over the lib exposing backwards compatible interface
// Having this increases the probability of upstreaming this rewrite, which I think should be a
// prerequisite for using it; we don't want to maintain our own fork of non-trivial tools that
// exist also in the official ACAP SDK.
use std::{
    env,
    fmt::{Display, Formatter},
    fs,
    fs::File,
    path::{Path, PathBuf},
};

use acap_build::{manifest::Manifest, AppBuilder, Architecture};
use anyhow::Context;
use clap::{Parser, ValueEnum};
use log::debug;

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
    #[clap(default_value_t, long, short)]
    build: BuildOption,
    // TODO: Look into mimicking the original interface exactly.
    /// Note: can be used more than once.
    #[clap(long)]
    meson_cross_file: Vec<PathBuf>,
    #[clap(long, short)]
    manifest: Option<PathBuf>,
    /// Note: can be used more than once.
    #[clap(long, short)]
    additional_file: Vec<PathBuf>,
    #[clap(long)]
    disable_manifest_validation: bool,
    #[clap(long)]
    disable_package_creation: bool,
    path: PathBuf,
}

impl Cli {
    fn read_app_name(manifest: &Path) -> anyhow::Result<String> {
        let manifest = fs::read_to_string(manifest)?;
        let manifest: Manifest = serde_json::from_str(&manifest)?;
        Ok(manifest.acap_package_conf.setup.app_name)
    }
    fn exec(self) -> anyhow::Result<()> {
        let arch: Architecture = env::var("OECORE_TARGET_ARCH")?.parse()?;
        // let staging_dir = TempDir::new("acap-build")?;
        let staging_dir = env::current_dir().unwrap().with_extension("tmp");
        if staging_dir.exists() {
            fs::remove_dir_all(&staging_dir)?;
        }
        fs::create_dir(&staging_dir)?;
        let manifest = match self.manifest {
            None => self.path.join("manifest.json"),
            Some(m) => self.path.join(m),
        };
        let app_name = Self::read_app_name(&manifest)?;
        let exe = self.path.join(&app_name);
        let license = self.path.join("LICENSE");
        let mut builder = AppBuilder::new(
            // staging_dir.path().to_path_buf(),
            staging_dir,
            arch,
            &app_name,
            &manifest,
            &exe,
            &license,
        )?;

        let lib = self.path.join("lib");
        if lib.exists() {
            builder.lib(&lib)?;
        }

        let html = self.path.join("html");
        if html.exists() {
            builder.html(&html)?;
        }

        for additional_file in self.additional_file {
            builder.additional_file(&self.path.join(additional_file))?;
        }

        let path = builder.build()?;

        Self::copy_artifacts(path.parent().unwrap(), &env::current_dir().unwrap(), &path)
    }

    fn copy_artifacts(src: &Path, dst: &Path, eap: &Path) -> anyhow::Result<()> {
        let (prefix, _) = eap
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .rsplit_once('_')
            .unwrap();
        let license = format!("{prefix}_LICENSE.txt");
        for file_name in [
            eap.file_name().unwrap().to_str().unwrap(),
            license.as_str(),
            "package.conf",
            "package.conf.orig",
            "param.conf",
        ] {
            fs::copy(src.join(file_name), dst.join(file_name))
                .with_context(|| format!("{file_name}: {src:?} -> {dst:?}"))?;
        }
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
