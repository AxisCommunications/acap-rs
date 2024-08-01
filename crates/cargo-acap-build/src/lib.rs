#![doc=include_str!("../README.md")]

use std::{
    fmt::Display,
    fs, mem,
    path::{Path, PathBuf},
};

pub use acap::Architecture;
pub use cargo::get_cargo_metadata;
pub use cargo_acap::Artifact;
use log::debug;

mod acap;
mod cargo;
mod cargo_acap;
mod command_utils;

pub struct AppBuilder {
    targets: Vec<Architecture>,
    args: Vec<String>,
    artifact_dir: Option<PathBuf>,
}

impl AppBuilder {
    pub fn from_targets<T, U>(targets: T) -> Self
    where
        T: IntoIterator<Item = U>,
        U: Into<Architecture>,
    {
        Self {
            targets: targets.into_iter().map(|t| t.into()).collect(),
            args: Vec::new(),
            artifact_dir: None,
        }
    }

    /// Add arguments that will be passed through to cargo.
    ///
    /// # Panics
    ///
    /// This function will panic if it detects one of the disallowed options:
    /// - `--artifact-dir`
    /// - `--target`
    ///
    /// <div class="warning">
    ///     Disallowing more options will not be considered a breaking change!
    /// </div>
    pub fn args<T, U>(&mut self, args: T) -> &mut Self
    where
        T: IntoIterator<Item = U>,
        U: Display,
    {
        self.args.extend(args.into_iter().map(|arg| {
            let arg: String = arg.to_string();
            let name = arg.split('=').next().unwrap();
            assert_ne!(name, "--artifact-dir");
            assert_ne!(name, "--target");
            arg
        }));
        self
    }

    /// Copy final artifacts to this directory.
    ///
    /// # Panics
    ///
    /// This function will panic if the artifact dir has already been set.
    pub fn artifact_dir<T>(&mut self, artifact_dir: T) -> &mut Self
    where
        T: Into<PathBuf>,
    {
        assert!(mem::replace(&mut self.artifact_dir, Some(artifact_dir.into())).is_none());
        self
    }

    pub fn execute(&mut self) -> anyhow::Result<Vec<Artifact>> {
        let args: Vec<_> = self.args.iter().map(String::as_str).collect();
        let mut artifacts = Vec::new();
        for target in &self.targets {
            artifacts.extend(cargo_acap::build_and_pack(*target, &args)?);
        }
        if let Some(artifact_dir) = self.artifact_dir.as_deref() {
            copy_final_artifacts(&artifacts, artifact_dir)?;
        }
        Ok(artifacts)
    }
}

fn copy_final_artifacts(artifacts: &[Artifact], acap_dir: &Path) -> anyhow::Result<()> {
    match fs::create_dir(acap_dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => Err(e)?,
    }
    for artifact in artifacts {
        let Artifact::Eap { path: src, name } = artifact else {
            debug!("Skipping artifact that is not an EAP: {artifact:?}");
            continue;
        };
        let dst = acap_dir.join(name);
        debug!("Copying `.eap` from {src:?} to {dst:?}");
        fs::copy(src, dst)?;
    }
    Ok(())
}
