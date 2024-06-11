use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use log::{debug, warn};
use url::Host;

use crate::{
    acap_utils,
    cargo_utils::{get_cargo_metadata, CargoMetadata, ExecutableArtifact},
    command_utils::RunWith,
    docker_utils::DockerOptions,
    ArchAbi,
};

pub struct StagedApp {
    pub stage_dir: PathBuf,
    pub additional_files: Vec<PathBuf>,
    pub name: String,
    pub executable: PathBuf,
}

impl StagedApp {
    pub fn try_new(
        ExecutableArtifact {
            arch,
            manifest_path,
            executable,
            out_dir,
            target_name,
        }: ExecutableArtifact,
    ) -> anyhow::Result<Self> {
        // Stage files that will be packed in their own directory that is outside of the source tree
        let app_dir = acap_utils::app_dir(&manifest_path).context("Not an app")?;
        let target_directory = get_cargo_metadata()?.target_directory;
        let stage_dir = ensure_empty_stage_dir(&target_directory, arch, &target_name)?;
        let additional_files = stage(&executable, out_dir.as_deref(), &app_dir, &stage_dir)?;
        Ok(StagedApp {
            stage_dir,
            additional_files,
            name: target_name,
            executable,
        })
    }

    fn pack(&self, docker_options: &DockerOptions, arch_abi: ArchAbi) -> anyhow::Result<PathBuf> {
        let Self {
            stage_dir,
            additional_files,
            ..
        } = self;

        let mut acap_build = std::process::Command::new("acap-build");
        acap_build.args(["--build", "no-build"]);
        for file in additional_files {
            acap_build.args(["--additional-file", file.to_str().unwrap()]);
        }
        acap_build.arg(".");

        let CargoMetadata { workspace_root, .. } = get_cargo_metadata().unwrap();
        let mut sh = docker_options.command(&workspace_root, "sh", false)?;

        let env_setup = match arch_abi {
            ArchAbi::Aarch64 => "environment-setup-cortexa53-crypto-poky-linux",
            ArchAbi::Armv7hf => "environment-setup-cortexa9hf-neon-poky-linux-gnueabi",
        };
        sh.args([
            "-c",
            &format!(
                ". /opt/axis/acapsdk/{env_setup} && cd {} && {acap_build:?}",
                stage_dir.display()
            ),
        ]);
        sh.run_with_inherited_stdout()?;
        let mut apps = Vec::new();
        for entry in std::fs::read_dir(stage_dir)? {
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

    pub fn sync(self, user: &str, pass: &str, host: &Host) -> anyhow::Result<String> {
        let mut files = HashMap::new();
        assert!(files
            .insert(PathBuf::from(&self.name), Some(self.executable))
            .is_none());
        for from in self.additional_files {
            files.insert(from.clone(), None);
        }
        acap_ssh_utils::sync_package(&self.stage_dir, files, user, pass, host, &self.name)?;
        Ok(self.name)
    }
}

pub fn stage_and_pack(
    docker_options: &DockerOptions,
    executables: Vec<ExecutableArtifact>,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut apps = Vec::new();
    for executable in executables {
        let manifest_path = &executable.manifest_path;
        let arch = executable.arch;
        if app_dir(manifest_path).is_none() {
            debug!("Manifest is not an app {manifest_path:?}");
            continue;
        };

        debug!("Found app, staging and packing...");
        let staged = StagedApp::try_new(executable)?;
        let app = staged.pack(docker_options, arch)?;
        apps.push(app);
    }

    Ok(apps)
}

pub fn app_dir(manifest_path: &Path) -> Option<PathBuf> {
    let Some(manifest_dir) = manifest_path.parent() else {
        warn!("Manifest has not parent {manifest_path:?}");
        return None;
    };
    if manifest_dir.join("manifest.json").is_file() {
        Some(manifest_dir.to_path_buf())
    } else {
        None
    }
}

fn ensure_empty_stage_dir(
    cargo_target_dir: &Path,
    target: ArchAbi,
    package_name: &str,
) -> anyhow::Result<PathBuf> {
    let mut stage_dir = cargo_target_dir.join(target.to_string());
    if !stage_dir.is_dir() {
        std::fs::create_dir(&stage_dir)?;
    }
    stage_dir.push(package_name);
    if stage_dir.is_dir() {
        std::fs::remove_dir_all(&stage_dir)?;
    }
    std::fs::create_dir(&stage_dir)?;
    Ok(stage_dir)
}

fn stage(
    executable: &Path,
    out_dir: Option<&Path>,
    package_dir: &Path,
    stage_dir: &Path,
) -> anyhow::Result<Vec<PathBuf>> {
    stage_executable(executable, stage_dir)?;
    // TODO: Consider disabling the default rule when a build script is exists
    // Except for the executable all files can be staged by the build script, so it could be
    // expected to stage all files.
    // Pros:
    // * `LICENSE` and `manifest.json` can be located anywhere, or even generated.
    // * Previously reserved path names can be used.
    // Cons:
    // * A predefine structure:
    //   * saves time by eliminating decisions, and
    //   * facilitates working on multiple projects
    // * Currently the presence of a `manifest.json` file is how we detect what crates are ACAP
    //   apps, and which are not.
    // Note that the original reason why `LICENSE` and `manifest.json` were required in the
    // manifest dir is that this is what the C examples look like.
    stage_special(package_dir, stage_dir)?;
    let mut additional_files = Vec::new();
    additional_files.extend(stage_other(package_dir, stage_dir)?);
    if let Some(out_dir) = out_dir {
        debug!("Copying other dynamic files from {out_dir:?}");
        additional_files.extend(stage_other(out_dir, stage_dir)?);
    }
    additional_files.sort();
    additional_files.dedup();
    Ok(additional_files)
}

fn stage_executable(executable: &Path, stage_dir: &Path) -> anyhow::Result<()> {
    let dst = stage_dir.join(executable.file_name().unwrap());
    if dst.exists() {
        anyhow::bail!(
            "{:?} already exists, ensure file does not also exist in `additional-files`",
            dst
        )
    }
    std::fs::copy(executable, dst)?;
    Ok(())
}

fn stage_special(app_dir: &Path, stage_dir: &Path) -> anyhow::Result<()> {
    // TODO: Consider creating LICENSE and manifest.json if they don't exist
    // TODO: When manifest.json is not generated, ensure appName and package.name match.
    for file_name in ["LICENSE", "manifest.json"] {
        let src = app_dir.join(file_name);
        if !src.exists() {
            bail!("`{file_name}` does not exist, this file is required")
        }
        let dst = stage_dir.join(file_name);
        if dst.exists() {
            bail!(
                "{file_name} already exists, ensure file does not also exist in `additional-files`"
            )
        }
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

fn stage_other(package_dir: &Path, stage_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut additional_files = Vec::new();
    let other = package_dir.join("additional-files");
    let entries = match std::fs::read_dir(&other) {
        Ok(t) => Ok(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            debug!("No additional-files directory exits");
            return Ok(Vec::new());
        }
        Err(e) => Err(e),
    }?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        copy_recursively(&path, &stage_dir.join(entry.file_name()))?;
        additional_files.push(path.strip_prefix(&other)?.to_path_buf());
    }
    Ok(additional_files)
}

fn copy_recursively(src: &PathBuf, dst: &PathBuf) -> anyhow::Result<()> {
    if src.is_file() {
        if dst.exists() {
            bail!("Path already exists {dst:?}");
        }
        std::fs::copy(src, dst)?;
        debug!("Created reg {dst:?}");
        return Ok(());
    }
    if !src.is_dir() {
        bail!("`{src:?}` is neither a file nor a directory");
    }
    match std::fs::create_dir(dst) {
        Ok(()) => {
            debug!("Created dir {dst:?}");
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        copy_recursively(&entry.path(), &dst.join(entry.file_name()))?;
    }
    Ok(())
}
