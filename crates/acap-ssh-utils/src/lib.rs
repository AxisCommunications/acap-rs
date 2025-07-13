#![forbid(unsafe_code)]
mod acap;

use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::{bail, Context};
use flate2::read::GzDecoder;
use log::{debug, warn};
use tar::Archive;
use url::Host;

use crate::acap::Manifest;

// TODO: Investigate if a Rust library can be used to replace `sshpass`, `ssh`, `scp`.
// This would make password handling easier and reduce the number of system dependencies that users
// have to install.
fn sshpass(pass: &str, program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new("sshpass");
    // TODO: Consider not passing the password as an argument
    cmd.arg(format!("-p{pass}"))
        .arg(program)
        // The ssh client will try keys until it finds one that works.
        // If it tries to many keys that fail it will be disconnected by the server.
        .args(["-o", "PubkeyAuthentication=no"])
        // TODO: Consider not disabling this as aggressively
        .args(["-o", "StrictHostKeyChecking=no"]);
    cmd
}

fn scp(src: &Path, user: &str, pass: &str, host: &Host, tgt: &str) -> std::process::Command {
    let mut cmd = sshpass(pass, "scp");
    cmd.arg("-p"); // Ensure temporary files become executable.
    cmd.arg(src);
    cmd.arg(format!("{user}@{host}:{tgt}"));
    cmd
}

fn ssh(user: &str, pass: &str, host: &Host) -> std::process::Command {
    let mut cmd = sshpass(pass, "ssh");
    cmd.arg("-x"); // Makes no difference when I have tested but seems to be the right thing to do.
    cmd.arg(format!("{user}@{host}"));
    cmd
}
fn spawn(mut cmd: std::process::Command) -> anyhow::Result<std::process::Child> {
    match cmd.spawn() {
        Ok(t) => Ok(t),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let program = cmd.get_program().to_string_lossy().to_string();
            Err(e).context(format!(
                "{program} not found, perhaps it must be installed."
            ))
        }
        Err(e) => Err(e.into()),
    }
}

trait RunWith {
    fn run_with_captured_stdout(self) -> anyhow::Result<String>;
    fn run_with_logged_stdout(self) -> anyhow::Result<()>;
    fn run_with_inherited_stdout(self) -> anyhow::Result<()>;
}

impl RunWith for std::process::Command {
    fn run_with_captured_stdout(mut self) -> anyhow::Result<String> {
        self.stdout(Stdio::piped());
        debug!("Spawning child {self:#?}...");
        let mut child = spawn(self)?;
        let mut stdout = child.stdout.take().unwrap();
        debug!("Waiting for child...");
        let status = child.wait()?;
        if !status.success() {
            bail!("Child failed: {status}");
        }
        let mut decoded = String::new();
        stdout.read_to_string(&mut decoded)?;
        Ok(decoded)
    }

    fn run_with_logged_stdout(mut self: std::process::Command) -> anyhow::Result<()> {
        self.stdout(Stdio::piped());
        debug!("Spawning child {self:#?}...");
        let mut child = spawn(self)?;
        let stdout = child.stdout.take().unwrap();

        let lines = BufReader::new(stdout).lines();
        for line in lines {
            let line = line?;
            if !line.is_empty() {
                debug!("Child said {:?}.", line);
            }
        }

        debug!("Waiting for child...");
        let status = child.wait()?;
        if !status.success() {
            bail!("Child failed: {status}");
        }
        Ok(())
    }

    fn run_with_inherited_stdout(mut self: std::process::Command) -> anyhow::Result<()> {
        debug!("Running command {self:#?}...");
        let status = self.status()?;
        if !status.success() {
            bail!("Child failed: {status}");
        }
        Ok(())
    }
}
struct RemoteTemporaryFile {
    path: String,
    ssh_rm: Option<std::process::Command>,
}

impl RemoteTemporaryFile {
    fn try_new(user: &str, pass: &str, host: &Host) -> anyhow::Result<Self> {
        let mut ssh_mktemp = ssh(user, pass, host);
        ssh_mktemp.arg("mktemp");
        let path = ssh_mktemp.run_with_captured_stdout()?.trim().to_string();
        let mut ssh_rm = ssh(user, pass, host);
        ssh_rm.arg("rm");
        ssh_rm.arg(&path);
        Ok(Self {
            path,
            ssh_rm: Some(ssh_rm),
        })
    }
}
impl Drop for RemoteTemporaryFile {
    fn drop(&mut self) {
        let ssh_rm = self.ssh_rm.take().unwrap();
        let path = &self.path;
        match ssh_rm.run_with_logged_stdout() {
            Ok(()) => debug!("Successfully cleaned up temporary file {path}."),
            Err(e) => warn!("Failed to clean up temporary file {path} because {e}."),
        }
    }
}
/// Run executable on device
///
/// `user` and `pass` are the credentials to use for the ssh connection.
/// `host` is the device to connect to.
/// `prog` is the path to the executable to run.
/// `env` is a map of environment variables to override on the remote host.
///
/// The function assumes that the user has already
/// - enabled SSH on the device,
/// - configured the SSH user with a password and the necessary permissions, and
/// - stopped the app.
pub fn run_other(
    prog: &Path,
    user: &str,
    pass: &str,
    host: &Host,
    env: HashMap<&str, &str>,
    args: &[&str],
) -> anyhow::Result<()> {
    let temp_file = RemoteTemporaryFile::try_new(user, pass, host)?;

    scp(prog, user, pass, host, &temp_file.path).run_with_logged_stdout()?;

    let mut exec = std::process::Command::new(&temp_file.path);
    exec.envs(env);
    exec.args(args);

    let mut ssh_exec = ssh(user, pass, host);
    ssh_exec.arg(format!("{exec:?}"));
    ssh_exec.run_with_inherited_stdout()?;

    Ok(())
}

// TODO: Consider abstracting away the difference between devices that support developer mode, and
//  those that don't.
/// Run ACAP app on device in a realistic manner.
///
/// `user` and `pass` are the credentials to use for the ssh connection.
/// `host` is the device to connect to.
/// `package` is the name of the ACAP app to emulate.
/// `env` is a map of environment variables to override on the remote host.
///
/// The function assumes that the user has already
/// - enabled SSH on the device,
/// - configured the SSH user with a password and the necessary permissions,
/// - installed the app, and
/// - stopped the app, if it was running.
pub fn run_package(
    user: &str,
    pass: &str,
    host: &Host,
    package: &str,
    env: HashMap<&str, &str>,
    args: &[&str],
) -> anyhow::Result<()> {
    let mut cd = std::process::Command::new("cd");
    cd.arg(format!("/usr/local/packages/{package}"));

    let mut exec = std::process::Command::new(format!("./{package}"));
    // TODO: Consider setting more environment variables
    exec.env("G_SLICE", "always-malloc");
    exec.envs(env);
    exec.args(args);

    let package_user = format!("acap-{package}");
    let exec_as_package = if user == package_user {
        let mut sh = std::process::Command::new("sh");
        sh.args(["-c", &format!("{exec:?}")]);
        sh
    } else {
        let mut su = std::process::Command::new("su");
        su.args(["--shell", "/bin/sh"]);
        su.args(["--command", &format!("{exec:?}")]);
        su.args([package_user]);
        su
    };

    // TODO: Consider giving user control over what happens with stdout when running concurrently.
    // The escaping of quotation marks is ridiculous, but it's automatic and empirically verifiable.
    let mut ssh_exec_as_package = ssh(user, pass, host);
    ssh_exec_as_package.arg(format!("{cd:?} && {exec_as_package:?}"));
    ssh_exec_as_package.run_with_inherited_stdout()?;
    Ok(())
}

/// Update ACAP app on device without installing it
///
/// - `package` the location of the `.eap` to upload.
/// - `user` and `pass` are the credentials to use for the ssh connection.
/// - `host` is the device to connect to.
///
/// The function assumes that the user has already
/// - enabled SSH on the device,
/// - configured the SSH user with a password and the necessary permissions,
/// - installed the app, and
/// - stopped the app, if it was running.
pub fn patch_package(package: &Path, user: &str, pass: &str, host: &Host) -> anyhow::Result<()> {
    // Not all files can be replaced, so we upload only the ones that can.
    // This archive will hold the files that will be uploded.
    let mut partial = tar::Builder::new(Vec::new());

    let mut full = Archive::new(GzDecoder::new(File::open(package)?));
    let mut app_name: Option<String> = None;
    for entry in full.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if path == Path::new("manifest.json") {
            let mut manifest = String::new();
            entry.read_to_string(&mut manifest)?;
            let manifest: Manifest = serde_json::from_str(&manifest)?;
            app_name = Some(manifest.acap_package_conf.setup.app_name)
        } else if path != Path::new("package.conf") {
            debug!("Adding {path:?} to new archive.");
            let mut buf = Vec::new();
            _ = entry.read_to_end(&mut buf)?;
            partial.append(entry.header(), &*buf)?;
        }
    }
    let Some(app_name) = app_name else {
        bail!("Could not find a manifest with the app name");
    };
    let package_dir = PathBuf::from("/usr/local/packages").join(app_name);
    // TODO: Copy only files that have been updated, e.g. as as decided by comparing the `mtime`.
    // TODO: Remove files that are no longer relevant.
    // Currently the error when an app is not installed is not very helpful:
    // tar: can't change directory to '/usr/local/packages/<APP_NAME>': No such file or directory
    // TODO: Better error when application is not installed
    let mut ssh_tar = ssh(user, pass, host);
    ssh_tar.args(["tar", "-xvC", package_dir.to_str().unwrap()]);

    ssh_tar.stdin(Stdio::piped());
    ssh_tar.stdout(Stdio::piped());
    debug!("Spawning {ssh_tar:#?}");
    let mut child = spawn(ssh_tar)?;

    child
        .stdin
        .take()
        .unwrap()
        .write_all(&partial.into_inner()?)?;

    let stdout = child.stdout.take().unwrap();
    for line in BufReader::new(stdout).lines() {
        let line = line?;
        if !line.is_empty() {
            debug!("Child said {line:?}");
        }
    }

    debug!("Waiting for child");
    let status = child.wait()?;
    if !status.success() {
        bail!("Child failed {status}");
    }
    Ok(())
}
