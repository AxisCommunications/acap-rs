/// Wrapper around the ACAP SDK, in particular `acap-build`.
use std::os::unix::fs::PermissionsExt;
use std::{
    env, fs,
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{bail, Context};
use command_utils::RunWith;
use log::{debug, info};
use serde::Serialize;
use serde_json::{ser::PrettyFormatter, Serializer, Value};

use crate::{
    cgi_conf::CgiConf, manifest::Manifest, package_conf::PackageConf, param_conf::ParamConf,
};

mod cgi_conf;
mod command_utils;
pub mod manifest;
mod package_conf;
mod param_conf;

// TODO: Find a better way to support reproducible builds
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

        let dst_exe = staging_dir.join(app_name);
        copy(exe, &dst_exe)?;
        let mut permissions = fs::metadata(&dst_exe)?.permissions();
        let mode = permissions.mode();
        permissions.set_mode(mode | 0o111);
        fs::set_permissions(&dst_exe, permissions)?;

        let builder = Self {
            staging_dir,
            arch,
            additional_files: Vec::new(),
        };

        copy(manifest, builder.manifest_file())?;
        copy(license, builder.license_file())?;

        Ok(builder)
    }

    fn license_file(&self) -> PathBuf {
        self.staging_dir.join("LICENSE")
    }

    fn manifest_file(&self) -> PathBuf {
        self.staging_dir.join("manifest.json")
    }

    pub fn additional_file(&mut self, src: &Path) -> anyhow::Result<&mut Self> {
        let dst = self.staging_dir.join(src.file_name().unwrap());
        if dst.exists() {
            bail!(
                "{} already exists",
                src.file_name().unwrap().to_string_lossy()
            );
        }
        copy_recursively(src, &dst)?;
        self.additional_files
            .push(PathBuf::from(src.file_name().unwrap()));

        Ok(self)
    }

    pub fn additional(&mut self, dir: &Path) -> anyhow::Result<&mut Self> {
        let entries = fs::read_dir(dir)?;
        for entry in entries {
            self.additional_file(&entry?.path())?;
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

    /// Build EAP and return its path
    pub fn build(&mut self) -> anyhow::Result<PathBuf> {
        let use_rust_acap_build = match env::var_os("ACAP_BUILD_RUST") {
            Some(v) if v.to_string_lossy() == "0" => Some(false),
            Some(v) if v.to_string_lossy() == "1" => Some(true),
            Some(v) => bail!("Expected ACAP_BUILD_RUST to be 0 or 1, but found {v:?}"),
            None => None,
        };
        if use_rust_acap_build.unwrap_or(cfg!(feature = "rust")) {
            // TODO: Implement manifest validation
            info!("Bypassing acap-build, manifest will not be validated");
            self.bypass_manifest2packageconf()?;
            self.bypass_eap_create()?;
        } else {
            debug!("Using acap-build");
            self.run_acap_build()?;
        }
        self.eap()
    }

    fn run_acap_build(&self) -> anyhow::Result<()> {
        let Self {
            staging_dir,
            arch,
            additional_files,
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

        let env_setup = match arch {
            Architecture::Aarch64 => "environment-setup-cortexa53-crypto-poky-linux",
            Architecture::Armv7hf => "environment-setup-cortexa9hf-neon-poky-linux-gnueabi",
        };
        sh.args([
            "-c",
            &format!(". /opt/axis/acapsdk/{env_setup} && {acap_build:?}"),
        ]);
        sh.run_with_logged_stdout()
    }

    fn eap(&self) -> anyhow::Result<PathBuf> {
        let mut apps = Vec::new();
        for entry in fs::read_dir(&self.staging_dir)? {
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

    fn get_pre_uninstall_script(manifest: &Manifest) -> Option<String> {
        manifest
            .acap_package_conf
            .uninstallation
            .as_ref()?
            .pre_uninstall_script
            .clone()
    }

    fn bypass_manifest2packageconf(&self) -> anyhow::Result<()> {
        let manifest_data = fs::read_to_string(self.manifest_file())?;
        let manifest: Manifest = serde_json::from_str(&manifest_data)?;

        let mut additional_files = self.additional_files.clone();
        if let Some(p) = Self::get_pre_uninstall_script(&manifest) {
            additional_files.push(PathBuf::from(p));
        }

        manifest2packageconf(&self.manifest_file(), &self.staging_dir, &additional_files)?;
        Ok(())
    }

    fn bypass_eap_create(&self) -> anyhow::Result<()> {
        todo!()
    }

    fn _run_eap_create(&self, sdk_root: &Path) -> anyhow::Result<()> {
        let manifest = fs::read_to_string(self.manifest_file())?;

        // This file is included in the eap so for as long as we want bit exact output we must
        // take care to serialize the manifest the same way as the python implementation.
        let mut manifest = serde_json::from_str::<Value>(&manifest).context(manifest)?;
        let Value::String(mut schema_version) = manifest
            .get("schemaVersion")
            .context("schemaVersion")?
            .clone()
        else {
            bail!("Expected schema version to be a string")
        };

        // Make it valid semver
        for _ in 0..(2 - schema_version.chars().filter(|&c| c == '.').count()) {
            schema_version.push_str(".0");
        }
        let schema_version = semver::Version::parse(&schema_version)?;
        if schema_version > semver::Version::new(1, 3, 0) {
            let setup = manifest
                .get_mut("acapPackageConf")
                .context("no key acapPackageConf in manifest")?
                .get_mut("setup")
                .context("no key setup in acapPackageConf")?;
            if let Some(a) = setup.get_mut("architecture") {
                if a != "all" && a != self.arch.nickname() {
                    bail!(
                        "Architecture in manifest ({a}) is not compatible with built target ({:?})",
                        self.arch
                    );
                }
            } else if let Value::Object(setup) = setup {
                debug!("Architecture not set in manifest, using {:?}", &self.arch);
                setup.insert(
                    "architecture".to_string(),
                    Value::String(self.arch.nickname().to_string()),
                );
            } else {
                bail!("Expected setup to be an object")
            }
        }

        let manifest_file = tempfile::NamedTempFile::new_in(&self.staging_dir)?;

        let mut serializer = Serializer::with_formatter(
            fs::File::create(manifest_file.path())?,
            PrettyFormatter::with_indent(b"    "),
        );
        manifest.serialize(&mut serializer)?;

        let sysroots = sdk_root.join("acapsdk/sysroots");
        let target_sysroot = sysroots.join(self.arch.nickname());
        let native_sysroot = sysroots.join("x86_64-pokysdk-linux");
        let mut cmd = std::process::Command::new(native_sysroot.join("usr/bin/eap-create.sh"));
        cmd.arg("-m")
            .arg(
                manifest_file
                    .as_ref()
                    .file_name()
                    .expect("path is a regular file and does not end with .."),
            )
            .arg("--no-validate")
            .env("OECORE_NATIVE_SYSROOT", native_sysroot)
            .env("SDKTARGETSYSROOT", target_sysroot)
            .current_dir(&self.staging_dir);
        cmd.run_with_logged_stdout()
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

impl FromStr for Architecture {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "aarch64" => Ok(Self::Aarch64),
            "arm" => Ok(Self::Armv7hf),
            _ => Err(anyhow::anyhow!("Unrecognized variant {s}")),
        }
    }
}

pub fn manifest2packageconf(
    manifest: &Path,
    output: &Path,
    additional_files: &[PathBuf],
) -> anyhow::Result<Vec<PathBuf>> {
    let mut created_files = Vec::new();

    let additional_files = additional_files
        .iter()
        .map(|f| output.join(f))
        .collect::<Vec<_>>();

    let manifest: Value = serde_json::from_reader(File::open(manifest)?)?;
    let package_conf = PackageConf::new_from_manifest(&manifest, output, &additional_files)?;
    let p = output.join(PackageConf::file_name());
    fs::write(&p, package_conf.to_string())?;
    created_files.push(p);

    let manifest = serde_json::from_value::<Manifest>(manifest)?;
    match ParamConf::from_manifest(&manifest) {
        Ok(v) => {
            let p = output.join(ParamConf::file_name());
            fs::write(&p, v.to_string())?;
            created_files.push(p);
        }
        Err(e) => {
            info!("Could not create param.conf because {e:?}")
        }
    };
    match CgiConf::from_manifest(&manifest) {
        Ok(v) => {
            let p = output.join(CgiConf::file_name());
            fs::write(&p, v.to_string()).unwrap();
            created_files.push(p);
        }
        Err(e) => {
            info!("Could not create cgi.conf because {e:?}")
        }
    };

    Ok(created_files)
}
