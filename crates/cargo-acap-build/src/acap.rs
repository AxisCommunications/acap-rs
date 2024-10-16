use std::os::unix::fs::PermissionsExt;
/// Wrapper around the ACAP SDK, in particular`acap-build`.
use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use log::debug;

use crate::command_utils::RunWith;

mod manifest;

fn copy<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> std::io::Result<u64> {
    let mut src = fs::File::open(src)?;
    let mut dst = fs::File::create(dst)?;
    std::io::copy(&mut src, &mut dst)
}

fn copy_recursively(src: &Path, dst: &Path) -> anyhow::Result<()> {
    if src.is_file() {
        if dst.exists() {
            bail!("Path already exists {dst:?}");
        }
        copy(src, dst)?;
        debug!("Created reg {dst:?}");
        return Ok(());
    }
    if !src.is_dir() {
        bail!("`{src:?}` is neither a file nor a directory");
    }
    match fs::create_dir(dst) {
        Ok(()) => {
            debug!("Created dir {dst:?}");
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        copy_recursively(&entry.path(), &dst.join(entry.file_name()))?;
    }
    Ok(())
}

pub struct AppBuilder {
    staging_dir: PathBuf,
    arch: Architecture,
    additional_files: Vec<PathBuf>,
}

impl AppBuilder {
    pub fn new(
        staging_dir: PathBuf,
        arch: Architecture,
        app_name: &str,
        manifest: &Path,
        exe: &Path,
        license: &Path,
    ) -> anyhow::Result<Self> {
        fs::create_dir(&staging_dir)?;

        copy(manifest, staging_dir.join("manifest.json"))?;
        copy(license, staging_dir.join("LICENSE"))?;

        let dst_exe = staging_dir.join(app_name);
        copy(exe, &dst_exe)?;
        let mut permissions = fs::metadata(&dst_exe)?.permissions();
        let mode = permissions.mode();
        permissions.set_mode(mode | 0o111);
        fs::set_permissions(&dst_exe, permissions)?;

        Ok(Self {
            staging_dir,
            arch,
            additional_files: Vec::new(),
        })
    }

    pub fn additional(&mut self, dir: &Path) -> anyhow::Result<&mut Self> {
        let entries = fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let src = entry.path();
            let dst = self.staging_dir.join(entry.file_name());
            if dst.exists() {
                bail!("{} already exists", entry.file_name().to_string_lossy());
            }
            copy_recursively(&src, &dst)?;
            self.additional_files
                .push(src.strip_prefix(dir)?.to_path_buf());
        }
        Ok(self)
    }
    pub fn lib(&mut self, dir: &Path) -> anyhow::Result<&mut Self> {
        let name = "lib";
        let dst = self.staging_dir.join(name);
        if dst.exists() {
            bail!("{name} already exists");
        }
        copy_recursively(dir, &dst)?;
        Ok(self)
    }

    pub fn html(&mut self, dir: &Path) -> anyhow::Result<&mut Self> {
        let name = "html";
        let dst = self.staging_dir.join(name);
        if dst.exists() {
            bail!("{name} already exists");
        }
        copy_recursively(dir, &dst)?;
        Ok(self)
    }

    pub fn build(&self) -> anyhow::Result<PathBuf> {
        let Self {
            staging_dir,
            additional_files,
            ..
        } = self;

        let mut acap_build = std::process::Command::new("acap-build");
        acap_build.args(["--build", "no-build"]);
        for file in additional_files {
            // Use `arg` twice to avoid fallible conversion from `&PathBuf` to `&str`.
            acap_build.arg("--additional-file");
            acap_build.arg(file);
        }
        acap_build.arg(".");

        let mut sh = std::process::Command::new("sh");
        sh.current_dir(staging_dir);

        let env_setup = match self.arch {
            Architecture::Aarch64 => "environment-setup-cortexa53-crypto-poky-linux",
            Architecture::Armv7hf => "environment-setup-cortexa9hf-neon-poky-linux-gnueabi",
        };
        sh.args([
            "-c",
            &format!(". /opt/axis/acapsdk/{env_setup} && {acap_build:?}"),
        ]);
        sh.run_with_logged_stdout()?;
        let mut apps = Vec::new();
        for entry in fs::read_dir(staging_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(extension) = path.extension() {
                if extension.to_str() == Some("eap") {
                    apps.push(path);
                }
            }
        }
        let mut apps = apps.into_iter();
        let app = apps.next().context("Expected at least one artifact")?;
        if let Some(second) = apps.next() {
            bail!("Built at least one unexpected .eap file {second:?}")
        }
        Ok(app)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Architecture {
    Aarch64,
    Armv7hf,
}

impl Architecture {
    pub fn triple(&self) -> &'static str {
        match self {
            Architecture::Aarch64 => "aarch64-unknown-linux-gnu",
            Architecture::Armv7hf => "thumbv7neon-unknown-linux-gnueabihf",
        }
    }

    pub fn nickname(&self) -> &'static str {
        match self {
            Self::Aarch64 => "aarch64",
            Self::Armv7hf => "armv7hf",
        }
    }
}
