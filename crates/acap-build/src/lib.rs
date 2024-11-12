/// Wrapper around the ACAP SDK, in particular `acap-build`.
use std::os::unix::fs::PermissionsExt;
use std::{
    env, fs,
    fs::File,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use anyhow::{anyhow, bail, Context};
use command_utils::RunWith;
use log::{debug, info};
use serde::Serialize;
use serde_json::{ser::PrettyFormatter, Serializer, Value};
use tempfile::NamedTempFile;

use crate::{
    cgi_conf::CgiConf, manifest::Manifest, package_conf::PackageConf, param_conf::ParamConf,
};

mod cgi_conf;
mod command_utils;
pub mod manifest;
mod package_conf;
mod param_conf;
mod static_package_conf;

// TODO: Find a better way to support reproducible builds
fn copy<P: AsRef<Path>, Q: AsRef<Path>>(
    src: P,
    dst: Q,
    copy_permissions: bool,
) -> anyhow::Result<()> {
    let src = src.as_ref();
    if src.is_symlink() {
        // FIXME: Copy symlink in Rust
        if !Command::new("cp")
            .arg("-d")
            .arg(src.as_os_str())
            .arg(dst.as_ref().as_os_str())
            .status()?
            .success()
        {
            bail!("Failed to copy symlink: {}", src.display());
        }
    } else if copy_permissions {
        fs::copy(src, dst)?;
    } else {
        let mut src = fs::File::open(src)?;
        let mut dst = fs::File::create(dst)?;
        std::io::copy(&mut src, &mut dst)?;
    }
    Ok(())
}

fn copy_recursively(src: &Path, dst: &Path, copy_permissions: bool) -> anyhow::Result<()> {
    if src.is_file() {
        if dst.exists() {
            bail!("Path already exists {dst:?}");
        }
        copy(src, dst, copy_permissions)?;
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
        copy_recursively(
            &entry.path(),
            &dst.join(entry.file_name()),
            copy_permissions,
        )?;
    }
    Ok(())
}

pub struct AppBuilder {
    staging_dir: PathBuf,
    arch: Architecture,
    additional_files: Vec<PathBuf>,
    copy_permissions: bool,
}

impl AppBuilder {
    pub fn new(
        staging_dir: PathBuf,
        arch: Architecture,
        app_name: &str,
        manifest: &Path,
        exe: &Path,
        license: &Path,
        copy_permissions: bool,
    ) -> anyhow::Result<Self> {
        fs::create_dir(&staging_dir).with_context(|| format!("{staging_dir:?}"))?;

        let dst_exe = staging_dir.join(app_name);

        copy(exe, &dst_exe, copy_permissions)?;

        let mut permissions = fs::metadata(&dst_exe)?.permissions();
        let mode = permissions.mode();
        permissions.set_mode(mode | 0o111);
        fs::set_permissions(&dst_exe, permissions)?;

        let builder = Self {
            staging_dir,
            arch,
            additional_files: Vec::new(),
            copy_permissions,
        };

        copy(manifest, builder.manifest_file(), copy_permissions)?;
        copy(license, builder.license_file(), copy_permissions)?;

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
        copy_recursively(src, &dst, self.copy_permissions)?;
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
        copy_recursively(dir, &dst, self.copy_permissions)?;
        Ok(self)
    }

    pub fn html(&mut self, dir: &Path) -> anyhow::Result<&mut Self> {
        let name = "html";
        let dst = self.staging_dir.join(name);
        if dst.exists() {
            bail!("{name} already exists");
        }
        copy_recursively(dir, &dst, self.copy_permissions)?;
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
            // Porting these would be a horrendous task if their full interface had to be
            // implemented,
            // so I think what I will do is merge them and pitch the program comprehensible as
            // a value add.
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

        manifest2packageconf(
            &self.manifest_file(),
            &self.staging_dir,
            &additional_files,
            self.arch,
        )?;
        Ok(())
    }

    fn create_package_conf(&self) -> anyhow::Result<()> {
        let param_conf = self.staging_dir.join(ParamConf::file_name());
        if !param_conf.exists() {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&param_conf)?;
            info!("Created an empty {:?}", param_conf);
        }

        // info!("Saving backup of package.conf");
        // let package_conf = self.staging_dir.join(PackageConf::file_name());
        // if package_conf.exists() {
        //     fs::rename(&param_conf, &package_conf.with_extension("conf.orig"))?;
        // }

        // TODO: More stuff

        Ok(())
    }

    fn do_make_the_tar(&self) -> anyhow::Result<()> {
        let mtime = match env::var_os("SOURCE_DATE_EPOCH") {
            Some(v) => v.into_string().map_err(|e| anyhow!("{e:?}"))?,
            None => String::from_utf8(Command::new("date").arg("+%s").output()?.stdout)?,
        };

        let manifest_data = fs::read_to_string(self.manifest_file())?;
        let manifest: Manifest = serde_json::from_str(&manifest_data)?;

        let package_name = manifest
            .acap_package_conf
            .setup
            .friendly_name
            .as_ref()
            .unwrap_or(&manifest.acap_package_conf.setup.app_name)
            .replace(' ', "_");
        let version = manifest.acap_package_conf.setup.version.replace('.', "_");
        let arch = manifest
            .acap_package_conf
            .setup
            .architecture
            .as_deref()
            .unwrap_or(self.arch.nickname());
        let tarb = format!("{package_name}_{version}_{arch}.eap");

        let mut other_files = self.additional_files.clone();
        if let Some(p) = Self::get_pre_uninstall_script(&manifest) {
            other_files.push(PathBuf::from(p));
        }

        let package_conf = PackageConf::new(
            &serde_json::from_str::<Value>(&manifest_data)?,
            &self.staging_dir,
            &other_files,
            self.arch,
        )?;

        let manifest_file = self.create_temporary_manifest()?;
        let manifest_file_name = manifest_file
            .path()
            .strip_prefix(&self.staging_dir)?
            .to_str()
            .unwrap()
            .to_string();

        let mut tar = Command::new("tar");
        tar.arg("--use-compress-program=gzip --no-name -9")
            .arg("--sort=name")
            .arg(format!("--mtime=@{mtime}"))
            .arg("--owner=0")
            .arg("--group=0")
            .arg("--numeric-owner")
            .arg("--create")
            .args(["--file", &tarb])
            .arg("--exclude-vcs")
            .arg("--exclude=*~")
            .arg("--format=gnu")
            .arg(format!(
                "--transform=flags=r;s|{manifest_file_name}|manifest.json|"
            ))
            .arg(&manifest.acap_package_conf.setup.app_name)
            .arg(PackageConf::file_name())
            .arg(ParamConf::file_name())
            .arg(self.license_file().file_name().unwrap().to_str().unwrap())
            .arg(manifest_file_name);
        // TODO: Pre upgrade script
        // TODO: Post install script
        tar.args(other_files);
        // TODO: httpd.conf.local.*
        // TODO: mime.types.local.*

        for dir in ["html", "declarations", "lib"] {
            if self.staging_dir.join(dir).exists() {
                tar.arg(dir);
            }
        }

        if let Some(v) = package_conf.http_cig_paths() {
            if !v.is_empty() {
                tar.arg(v);
            }
        }
        tar.arg("--verbose");
        tar.current_dir(&self.staging_dir);
        tar.run_with_logged_stdout()?;
        Ok(())
    }

    fn bypass_eap_create(&self) -> anyhow::Result<()> {
        self.create_package_conf()?;
        self.do_make_the_tar()?;
        Ok(())
    }

    fn create_temporary_manifest(&self) -> anyhow::Result<NamedTempFile> {
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
        Ok(manifest_file)
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
    arch: Architecture,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut created_files = Vec::new();

    let additional_files = additional_files
        .iter()
        .map(|f| output.join(f))
        .collect::<Vec<_>>();

    let manifest: Value = serde_json::from_reader(File::open(manifest)?)?;
    let package_conf = PackageConf::new(&manifest, output, &additional_files, arch)?;
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
