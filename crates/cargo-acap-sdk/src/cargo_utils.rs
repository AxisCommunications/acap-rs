use std::{collections::HashMap, path::PathBuf};

use anyhow::Context;
use log::{debug, error, warn};
use serde::Deserialize;

use crate::{
    command_utils::RunWith, docker_utils::DockerOptions, ArchAbi, BuildOptions, Verbosity,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "reason")]
pub enum JsonMessage {
    #[serde(rename = "compiler-artifact")]
    CompilerArtifact {
        package_id: String,
        manifest_path: String,
        executable: Option<String>,
        target: Target,
    },
    // We don't care about the content of these for now, but include them so that the parsing
    // succeeds.
    #[serde(rename = "compiler-message")]
    CompilerMessage,
    #[serde(rename = "build-script-executed")]
    BuildScriptExecuted { package_id: String, out_dir: String },
    #[serde(rename = "build-finished")]
    BuildFinished,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CargoMetadata {
    pub target_directory: PathBuf,
    pub workspace_root: PathBuf,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Target {
    pub name: String,
}

pub fn get_cargo_metadata() -> anyhow::Result<CargoMetadata> {
    let mut cargo = std::process::Command::new("cargo");
    cargo.arg("metadata");
    let metadata = String::from_utf8(cargo.output()?.stdout)?;
    let metadata: CargoMetadata = serde_json::from_str(&metadata)?;
    Ok(metadata)
}

pub fn cargo_home() -> anyhow::Result<PathBuf> {
    let Some(cargo_home) = std::env::var_os("CARGO_HOME") else {
        return Ok(home::home_dir()
            .context("Could not determine home_dir")?
            .join(".cargo"));
    };
    let cargo_home = PathBuf::from(cargo_home);
    if cargo_home.is_absolute() {
        Ok(cargo_home)
    } else {
        Ok(std::env::current_dir()?.join(cargo_home))
    }
}

pub struct ExecutableArtifact {
    pub arch: ArchAbi,
    pub manifest_path: PathBuf,
    pub executable: PathBuf,
    pub out_dir: Option<PathBuf>,
    // We follow the convention that all of these have the same name:
    // * cargo package
    // * ACAP app
    // * executable file
    // This is not true when compiling tests, so we get the name explicitly.
    pub target_name: String,
}
pub fn build_all(
    docker_options: &DockerOptions,
    build_options: BuildOptions,
    test: bool,
    profile: &str,
) -> anyhow::Result<Vec<ExecutableArtifact>> {
    let mut executables = Vec::new();
    for target in build_options.targets() {
        executables.extend(build_one(
            docker_options,
            build_options.package.as_deref(),
            target,
            &build_options.verbosity,
            test,
            profile,
        )?);
    }
    Ok(executables)
}

fn build_one(
    docker_options: &DockerOptions,
    package_name: Option<&str>,
    target: ArchAbi,
    verbosity: &Verbosity,
    test: bool,
    profile: &str,
) -> anyhow::Result<Vec<ExecutableArtifact>> {
    debug!("Building {package_name:?} for {target}");
    let target: crate::Target = target.into();
    let mut cargo = std::process::Command::new("cargo");
    cargo.env("RUST_LOG_STYLE", "always");
    if let Some(arg) = verbosity.arg() {
        cargo.arg(arg);
    }
    if test {
        cargo.arg("test");
        cargo.arg("--no-run");
    } else {
        cargo.arg("build");
    }
    cargo.args(["--profile", profile]);
    cargo.args(["--target", target.triple()]);
    if let Some(package_name) = package_name {
        cargo.args(["--package", package_name]);
    }
    // TODO: Fix colorized output
    // I think I added this a long time ago because it ensured that the messages printed to stderr
    // by cargo are colored despite running as a subprocess, but that clearly is not happening and
    // I cannot find a commit where it actually works...
    // Running docker with `--tty` enables the color but merges stdout and stderr, so one way to
    // get color could be to guess the source of a message and relay human messages in real time.
    cargo.args(["--message-format", "json-render-diagnostics"]);

    let CargoMetadata { workspace_root, .. } = get_cargo_metadata().unwrap();
    let mut sh = docker_options.command(&workspace_root, "sh", false)?;
    sh.args(["-c", &format!("{cargo:?}")]);
    let mut messages = Vec::new();
    sh.run_with_processed_stdout(|line| {
        match line {
            Ok(line) => match serde_json::from_str::<JsonMessage>(&line) {
                Ok(message) => messages.push(message),
                Err(e) => error!("Could not parse line because {e}"),
            },
            Err(e) => {
                error!("Could not take line because {e}");
                return Ok(());
            }
        }
        Ok(())
    })?;

    let mut out_dirs = HashMap::new();
    let mut artifacts = Vec::new();
    for m in messages {
        match m {
            JsonMessage::CompilerArtifact {
                package_id,
                executable,
                manifest_path,
                target: Target { name: target_name },
                ..
            } => {
                if let Some(exe) = executable {
                    artifacts.push(ExecutableArtifact {
                        arch: target.into(),
                        executable: exe.into(),
                        manifest_path: manifest_path.into(),
                        out_dir: out_dirs.remove(&package_id),
                        target_name,
                    });
                }
            }
            JsonMessage::CompilerMessage { .. } => {
                // We expect these to be rendered to stderr when `--message-format` is
                // set to `json-render-diagnostics`, as opposed to `json`.
                error!("Received compiler-message")
            }
            JsonMessage::BuildFinished { .. } => {
                debug!("Received build-finished message")
            }
            JsonMessage::BuildScriptExecuted {
                package_id,
                out_dir,
            } => {
                debug!("Received build-script-executed message for {package_id}");
                if let Some(out_dir) = out_dirs.insert(package_id, out_dir.into()) {
                    warn!("Discarding out dir {out_dir:?}")
                }
            }
        }
    }
    Ok(artifacts)
}
