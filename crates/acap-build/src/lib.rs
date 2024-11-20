#![forbid(unsafe_code)]
//! Library for creating Embedded Application Packages (EAPs).
use std::{
    env, ffi::OsString, fs, io::Write, os::unix::fs::PermissionsExt, path::Path, process::Command,
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
    if src.is_symlink() {
        // FIXME: Copy symlink in Rust
        if !Command::new("cp")
            .arg("-dn")
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
    additional_files: Vec<String>,
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
            additional_files: Vec::new(),
            default_architecture,
        })
    }

    /// Add files that don't fit any other category to the EAP.
    pub fn add_additional(&mut self, path: &Path) -> anyhow::Result<&mut Self> {
        let name = path
            .file_name()
            .context("file has no name")?
            .to_str()
            .context("file name is not a string")?
            .to_string();
        let dst = self.staging_dir.join(&name);
        if dst.exists() {
            bail!("{name} already exists");
        }
        copy_recursively(path, &dst, self.preserve_permissions)?;
        self.additional_files.push(name);
        Ok(self)
    }

    // acap-build does not expose this and the docs don't mention it,
    // but eap-create.sh would add it if it exists.
    // TODO: Consider removing
    /// Add event declarations to the EAP.
    pub fn add_declarations(&mut self, dir: &Path) -> anyhow::Result<&mut Self> {
        let name = "declarations";
        let dst = self.staging_dir.join(name);
        if dst.exists() {
            bail!("{name} already exists");
        }
        copy_recursively(dir, &dst, self.preserve_permissions)?;
        Ok(self)
    }

    // TODO: Consider making mandatory
    /// Add the **mandatory** executable to the EAP.
    pub fn add_exe(&mut self, reg: &Path) -> anyhow::Result<&mut Self> {
        let dst = self.staging_dir.join(&self.app_name);
        copy(reg, &dst, self.preserve_permissions)?;
        if !self.preserve_permissions {
            let mut permissions = fs::metadata(&dst)?.permissions();
            let mode = permissions.mode();
            permissions.set_mode(mode | 0o111);
            fs::set_permissions(&dst, permissions)?;
        }
        Ok(self)
    }

    /// Add an embedded web page to the EAP.
    pub fn add_html(&mut self, dir: &Path) -> anyhow::Result<&mut Self> {
        let name = "html";
        let dst = self.staging_dir.join(name);
        if dst.exists() {
            bail!("{name} already exists");
        }
        copy_recursively(dir, &dst, self.preserve_permissions)?;
        Ok(self)
    }

    // TODO: Consider making mandatory
    /// Add the **mandatory** open source attributions to the EAP.
    pub fn add_license(&mut self, reg: &Path) -> anyhow::Result<&mut Self> {
        copy(
            reg,
            self.staging_dir.join("LICENSE"),
            self.preserve_permissions,
        )?;
        Ok(self)
    }

    /// Add shared libraries to the EAP.
    pub fn add_lib(&mut self, dir: &Path) -> anyhow::Result<&mut Self> {
        let name = "lib";
        let dst = self.staging_dir.join(name);
        if dst.exists() {
            bail!("{name} already exists");
        }
        copy_recursively(dir, &dst, self.preserve_permissions)?;
        Ok(self)
    }

    // A backwards compatible program needs to read the manifest to know the name of the executable
    // and this method is added only to facilitate for that.
    // TODO: Consider removing this
    /// Return the short name of the app.
    pub fn app_name(&self) -> &str {
        self.app_name.as_str()
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
            additional_files,
            manifest,
            ..
        } = self;

        fs::File::create_new(staging_dir.join("manifest.json"))?
            .write_all(manifest.try_to_string()?.as_bytes())?;

        let mut acap_build = Command::new("acap-build");
        acap_build.args(["--build", "no-build"]);
        for file in additional_files {
            // Use `arg` twice to avoid fallible conversion from `&PathBuf` to `&str`.
            acap_build.arg("--additional-file");
            acap_build.arg(file);
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
            additional_files,
            default_architecture,
            app_name,
            ..
        } = self;
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

        // Copy files named in manifest.
        // The term "additional files" is used to mean files requested directly.
        // The term "other files" is used to mean additional files plus any files named in the
        // manifest.
        let other_files = additional_files;
        match manifest.try_find_pre_uninstall_script() {
            Ok(_) => {
                // TODO: Add support for pre-uninstall and post-install scripts.
                bail!("The pre-uninstall script is not supported yet.")
            }
            Err(json_ext::Error::KeyNotFound(k)) => {
                debug!("No {k}, skipping pre uninstall script")
            }
            Err(e) => return Err(e.into()),
        }

        // Generate derived files
        let package_conf =
            PackageConf::new(&manifest, &other_files, default_architecture)?.to_string();
        fs::File::create_new(staging_dir.join("package.conf"))?
            .write_all(package_conf.as_bytes())?;

        let param_conf = match ParamConf::new(&manifest)? {
            None => {
                // If there is no param.conf, `eap-create.sh` creates one
                debug!("Creating empty param.conf");
                String::new()
            }
            Some(v) => v.to_string(),
        };
        fs::File::create_new(staging_dir.join("param.conf"))?.write_all(param_conf.as_bytes())?;

        match CgiConf::new(&manifest)? {
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
            .arg("--exclude-vcs")
            .arg(&app_name)
            .arg("package.conf")
            .arg("param.conf")
            .arg("LICENSE")
            .arg("manifest.json");

        // TODO: Add support for the post-install script

        tar.args(&other_files);

        // TODO: Consider implementing support for `httpd.conf.local.*` and `mime.types.local.*`.

        for dir in ["html", "declarations", "lib", "cgi.conf"] {
            if staging_dir.join(dir).exists() {
                tar.arg(dir);
            }
        }

        tar.arg("--verbose");
        tar.current_dir(staging_dir);
        tar.run_with_logged_stdout()?;

        Ok(OsString::from(eap_file_name))
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
