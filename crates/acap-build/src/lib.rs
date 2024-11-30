#![forbid(unsafe_code)]
//! Library for creating Embedded Application Packages (EAPs).
use std::{
    collections::HashSet,
    env,
    ffi::OsString,
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use anyhow::{anyhow, bail, Context};
use command_utils::RunWith;
use log::{debug, info};
use semver::Version;
use serde_json::Value;

use crate::files::{
    cgi_conf::CgiConf, manifest::Manifest, package_conf::PackageConf, param_conf::ParamConf,
};

mod command_utils;
mod json_ext;

mod files;

// TODO: Find a better way to support reproducible builds
fn copy<P: AsRef<Path>, Q: AsRef<Path>>(
    src: P,
    dst: Q,
    copy_permissions: bool,
) -> anyhow::Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    if dst.symlink_metadata().is_ok() {
        bail!("Path already exists {dst:?}");
    }
    if src.is_symlink() {
        // FIXME: Copy symlink in Rust
        let mut cp = Command::new("cp");

        if copy_permissions {
            cp.arg("--preserve=mode");
        }

        cp.arg("-dn").arg(src.as_os_str()).arg(dst.as_os_str());

        if !cp.status()?.success() {
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
    if !src.is_dir() {
        copy(src, dst, copy_permissions)?;
        debug!("Created reg {dst:?}");
        return Ok(());
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

enum AcapBuildImpl {
    Reference,
    Equivalent,
}

impl AcapBuildImpl {
    fn from_env_or_default() -> anyhow::Result<Self> {
        Ok(match env::var_os("ACAP_BUILD_IMPL") {
            Some(v) if v.to_string_lossy() == "reference" => Self::Reference,
            Some(v) if v.to_string_lossy() == "equivalent" => Self::Equivalent,
            Some(v) => {
                bail!("Expected ACAP_BUILD_IMPL to be 'reference' or 'equivalent', but found {v:?}")
            }
            None => Self::Reference,
        })
    }
}

pub struct AppBuilder<'a> {
    preserve_permissions: bool,
    staging_dir: &'a Path,
    manifest: Manifest,
    files: Vec<String>,
    default_architecture: Architecture,
    app_name: String,
}

impl<'a> AppBuilder<'a> {
    pub fn new(
        preserve_permissions: bool,
        staging_dir: &'a Path,
        manifest: &Path,
        default_architecture: Architecture,
    ) -> anyhow::Result<Self> {
        let manifest: Value = serde_json::from_reader(fs::File::open(manifest)?)?;
        let manifest = Manifest::new(manifest, default_architecture)?;
        let app_name = manifest.try_find_app_name()?.to_string();
        Ok(Self {
            preserve_permissions,
            staging_dir,
            manifest,
            app_name,
            files: Vec::new(),
            default_architecture,
        })
    }

    /// Add a file to the EAP.
    pub fn add(&mut self, path: &Path) -> anyhow::Result<&mut Self> {
        let name = path
            .file_name()
            .context("file has no name")?
            .to_str()
            .context("file name is not a string")?;
        let dst = self.add_as(path, name)?;
        if name == self.app_name && !self.preserve_permissions {
            let mut permissions = fs::metadata(&dst)?.permissions();
            let mode = permissions.mode();
            permissions.set_mode(mode | 0o111);
            fs::set_permissions(&dst, permissions)?;
        }
        Ok(self)
    }

    /// Add all files in a directory to the EAP.
    pub fn add_from(&mut self, dir: &Path) -> anyhow::Result<&mut Self> {
        let mut entries = fs::read_dir(dir)?
            .map(|res| res.map(|e| e.path()))
            .collect::<std::io::Result<Vec<PathBuf>>>()?;
        entries.sort();
        for entry in entries {
            let name = entry
                .file_name()
                .context("file has no name")?
                .to_str()
                .context("file name is not a string")?;
            self.add_as(&entry, name)?;
        }
        Ok(self)
    }

    // TODO: Remove the file system copy
    pub fn add_as(&mut self, path: &Path, name: &str) -> anyhow::Result<PathBuf> {
        let dst = self.staging_dir.join(name);
        if dst.symlink_metadata().is_ok() {
            bail!("Cannot add {path:?} because {name} already exists");
        }
        copy_recursively(path, &dst, self.preserve_permissions)?;
        self.files.push(name.to_string());
        debug!("Added {name} from {path:?}");
        Ok(dst)
    }

    /// Build the EAP and return its path.
    pub fn build(self) -> anyhow::Result<OsString> {
        match AcapBuildImpl::from_env_or_default()? {
            AcapBuildImpl::Reference => {
                debug!("Using acap-build");
                self.build_foreign()
            }
            AcapBuildImpl::Equivalent => {
                // TODO: Implement validation.
                info!("Bypassing acap-build, manifest will not be validated");
                self.build_native()
            }
        }
    }

    fn build_foreign(self) -> anyhow::Result<OsString> {
        let Self {
            staging_dir,
            default_architecture,
            manifest,
            ..
        } = &self;

        fs::File::create_new(staging_dir.join("manifest.json"))
            .context("creating manifest.json")?
            .write_all(manifest.try_to_string()?.as_bytes())?;

        let mut acap_build = Command::new("acap-build");
        acap_build.args(["--build", "no-build"]);
        for file in self.additional_files() {
            acap_build.args(["--additional-file", file]);
        }
        acap_build.arg(".");

        let mut sh = Command::new("sh");
        sh.current_dir(staging_dir);

        let env_setup = match default_architecture {
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
        Ok(app.file_name().context("file has no name")?.to_os_string())
    }

    fn build_native(self) -> anyhow::Result<OsString> {
        let Self {
            staging_dir,
            manifest,

            default_architecture,
            app_name,
            ..
        } = &self;
        let mtime = match env::var_os("SOURCE_DATE_EPOCH") {
            Some(v) => v.into_string().map_err(|e| anyhow!("{e:?}"))?,
            None => String::from_utf8(Command::new("date").arg("+%s").output()?.stdout)?,
        };

        // Compute file name
        let package_name = match manifest.try_find_friendly_name() {
            Ok(v) => v,
            Err(json_ext::Error::KeyNotFound(_)) => app_name.as_str(),
            Err(e) => return Err(e.into()),
        }
        .replace(' ', "_");
        let Version {
            major,
            minor,
            patch,
            ..
        } = manifest.try_find_version().context("no version")?.parse()?;

        let arch = match manifest.try_find_architecture() {
            Ok(v) => v,
            Err(json_ext::Error::KeyNotFound(_)) => default_architecture.nickname(),
            Err(e) => return Err(e.into()),
        };
        let eap_file_name = format!("{package_name}_{major}_{minor}_{patch}_{arch}.eap");

        // Generate derived files
        let package_conf =
            PackageConf::new(manifest, &self.other_files(), *default_architecture)?.to_string();
        fs::File::create_new(staging_dir.join("package.conf"))?
            .write_all(package_conf.as_bytes())?;

        let param_conf = match ParamConf::new(manifest)? {
            None => {
                // If there is no param.conf, `eap-create.sh` creates one
                debug!("Creating empty param.conf");
                String::new()
            }
            Some(v) => v.to_string(),
        };
        fs::File::create_new(staging_dir.join("param.conf"))?.write_all(param_conf.as_bytes())?;

        match CgiConf::new(manifest)? {
            None => {
                debug!("Skipping cgi.conf")
            }
            Some(cgi_conf) => {
                fs::File::create_new(staging_dir.join("cgi.conf"))?
                    .write_all(cgi_conf.to_string().as_bytes())?;
            }
        }

        // This file is included in the EAP, so for as long as we want bit-exact output, we must
        // take care to serialize the manifest the same way as the python implementation.
        let manifest_file = staging_dir.join("manifest.json");
        fs::File::create_new(&manifest_file)?.write_all(manifest.try_to_string()?.as_bytes())?;
        // Replicate the permissions that temporary files get by default.
        let mut permissions = fs::metadata(&manifest_file)?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&manifest_file, permissions)?;

        // Create the archive
        let mut tar = Command::new("tar");
        tar.args(["--exclude", "*~"])
            .args(["--file", &eap_file_name])
            .args(["--format", "gnu"])
            .args(["--group", "0"])
            .args(["--mtime", &format!("@{mtime}")])
            .args(["--owner", "0"])
            .args(["--sort", "name"])
            .args(["--use-compress-program", "gzip --no-name -9"])
            .arg("--create")
            .arg("--numeric-owner")
            .arg("--exclude-vcs");

        for name in self.section_1_files() {
            if staging_dir.join(name).symlink_metadata().is_ok() {
                tar.arg(name);
            }
        }

        tar.args(self.other_files());

        // TODO: Consider implementing support for `httpd.conf.local.*` and `mime.types.local.*`.

        for name in self.section_4_files() {
            if staging_dir.join(name).symlink_metadata().is_ok() {
                tar.arg(name);
            }
        }

        tar.arg("--verbose");
        tar.current_dir(staging_dir);
        tar.run_with_logged_stdout()?;

        Ok(OsString::from(eap_file_name))
    }

    // These sections are probably relevant only for the equivalent and reference implementations;
    // Once unpacked on device the order of files or the reason they were included is not important
    // (even though some files are nonetheless treated specially).
    // The sections don't have any semantics, they are just partitions that can be composed to
    // create meaningful or useful lists of names.

    fn section_1_files(&self) -> Vec<&str> {
        [
            Some(self.app_name.as_str()),
            Some("package.conf"),
            Some("param.conf"),
            Some("LICENSE"),
            Some("manifest.json"),
            self.manifest.try_find_post_install_script().ok(),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn section_2_files(&self) -> Vec<&str> {
        let known_files: HashSet<_> = [
            self.section_1_files(),
            self.section_3_files(),
            self.section_4_files(),
        ]
        .into_iter()
        .flatten()
        .collect();

        self.files
            .iter()
            .map(String::as_str)
            .filter(|f| !known_files.contains(f))
            .collect()
    }

    fn section_3_files(&self) -> Vec<&str> {
        [self.manifest.try_find_pre_uninstall_script().ok()]
            .into_iter()
            .flatten()
            .collect()
    }

    fn section_4_files(&self) -> Vec<&str> {
        ["html", "declarations", "lib", "cgi.conf"]
            .into_iter()
            .collect()
    }

    /// Additional files for the reference implementation.
    fn additional_files(&self) -> Vec<&str> {
        self.section_2_files()
    }

    /// Other files for the `package.conf` file.
    fn other_files(&self) -> Vec<&str> {
        [self.section_2_files(), self.section_3_files()].concat()
    }

    /// Return the name of files that must be added using [`Self::add`].
    pub fn mandatory_files(&self) -> Vec<String> {
        [
            Some(self.app_name.as_str()),
            Some("LICENSE"),
            self.manifest.try_find_post_install_script().ok(),
            self.manifest.try_find_pre_uninstall_script().ok(),
        ]
        .into_iter()
        .flatten()
        .map(str::to_string)
        .collect()
    }

    /// Return the name of files that should be added using [`Self::add`].
    pub fn optional_files(&self) -> Vec<String> {
        ["html", "declarations", "lib"]
            .into_iter()
            .map(str::to_string)
            .collect()
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
