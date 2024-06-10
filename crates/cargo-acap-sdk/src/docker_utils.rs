use log::debug;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::bail;
use libc::{getegid, geteuid};

use crate::{cargo_utils, command_utils::RunWith};

// TODO: Investigate if argument parsing can be tidier
// TODO: Consider using an enum to represent configuration
// An enum could encode that only one of the sources, or no source, is valid at one time.
// It could also encode that a docker-image has been created from a docker-file.
#[derive(clap::Args, Debug, Clone)]
pub struct DockerOptions {
    /// Build exe and pack eap in a container built from this Dockerfile.
    #[arg(long, conflicts_with = "docker_image", conflicts_with = "no_docker")]
    docker_file: Option<PathBuf>,
    /// Build exe and pack eap in a container created from this image.
    #[arg(long, conflicts_with = "no_docker")]
    docker_image: Option<String>,
    /// Build exe and pack eap on host.
    #[arg(long)]
    no_docker: bool,
    /// Set environment variables
    #[arg(long)]
    docker_env: Vec<String>,
}

impl DockerOptions {
    pub fn build_docker_image(&mut self) -> anyhow::Result<()> {
        match (self.docker_file.take(), &self.docker_image, self.no_docker) {
            (None, None, false) => {
                debug!("No docker options specified, building the default image");
                self.docker_image = Some(build_image_without_context(include_str!(
                    "../../../Dockerfile"
                ))?);
            }
            (None, None, true) => debug!("--no-docker set, not building any image"),
            (Some(path), None, false) => {
                debug!("Building docker image from file");
                self.docker_image = Some(build_image_with_context(&path)?);
            }
            (None, Some(_), false) => {
                debug!("--docker-image set, using image as is")
            }
            _ => panic!("Got two or more mutually exclusive arguments"),
        }
        Ok(())
    }

    pub fn command(
        &self,
        current_dir: &Path,
        program: &str,
        interactive_tty: bool,
    ) -> anyhow::Result<std::process::Command> {
        if self.docker_image.is_some() {
            self.docker_command(current_dir, Some(program), interactive_tty)
        } else {
            assert!(self.docker_file.is_none());
            Ok(host_command(current_dir, program, &self.docker_env))
        }
    }

    /// It is the responsibility of the user to ensure that `docker_image.is_some()` e.g. by
    /// first calling `build_docker_image`.
    pub fn docker_command(
        &self,
        current_dir: &Path,
        program: Option<&str>,
        interactive_tty: bool,
    ) -> anyhow::Result<std::process::Command> {
        let mut docker = std::process::Command::new("docker");
        docker.arg("run");
        docker.arg("--rm");

        if interactive_tty {
            docker.args(["--interactive", "--tty"]);
        }

        let uid = unsafe { geteuid() };
        let gid = unsafe { getegid() };
        docker.args(["--user", &format!("{uid}:{gid}")]);

        let current_dir = current_dir.display().to_string();
        docker.args(["--volume", &format!("{current_dir}:{current_dir}")]);
        docker.args(["--workdir", &current_dir]);

        // Only needed for Cargo commands, but
        // * it is easier to always set it, and
        // * it is probably harmless when not needed.
        let cargo_home = cargo_utils::cargo_home()?.display().to_string();
        docker.args(["--volume", &format!("{cargo_home}:{cargo_home}",)]);
        docker.args(["--env", &format!("CARGO_HOME={cargo_home}")]);

        let passwd = "/etc/passwd";
        if PathBuf::from(passwd).is_file() {
            docker.args(["--volume", &format!("{passwd}:{passwd}:ro")]);
        }

        for e in &self.docker_env {
            docker.args(["--env", e]);
        }

        let docker_image = self.docker_image.as_ref().unwrap().clone();
        docker.arg(docker_image);

        if let Some(program) = program {
            docker.arg(program);
        }

        Ok(docker)
    }
}
pub fn build_image_with_context(dockerfile: &Path) -> anyhow::Result<String> {
    let dockerfile = dockerfile.to_path_buf().canonicalize()?;
    let tag = dockerfile
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_lowercase();
    let mut docker = std::process::Command::new("docker");
    docker.arg("build");
    docker.args(["--file", dockerfile.to_str().unwrap()]);
    docker.args(["--tag", &tag]);
    docker.arg(dockerfile.parent().unwrap());
    docker.run_with_inherited_stdout()?;
    Ok(tag)
}

pub fn build_image_without_context(dockerfile: &str) -> anyhow::Result<String> {
    let tag = "cargo-acap-sdk-default".to_string();
    let mut docker = std::process::Command::new("docker");
    docker.arg("build");
    docker.args(["--tag", &tag]);
    docker.arg("-");
    docker.stdin(Stdio::piped());
    debug!("Spawning {docker:#?}");
    let mut child = docker.spawn()?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(dockerfile.as_bytes())?;
    let status = child.wait()?;
    if !status.success() {
        bail!("Child failed: {status}")
    }
    Ok(tag)
}

pub fn host_command(current_dir: &Path, program: &str, env: &[String]) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    cmd.current_dir(current_dir);
    for e in env {
        match e.split_once('=') {
            None => {
                let k = e;
                if let Some(v) = std::env::var_os(k) {
                    cmd.env(k, v);
                }
            }
            Some((k, v)) => {
                cmd.env(k, v);
            }
        }
    }
    cmd
}
