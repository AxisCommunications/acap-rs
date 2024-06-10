use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::bail;
use log::{debug, warn};
use url::Host;

fn sshpass(pass: &str, program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new("sshpass");
    // TODO: Consider not passing the password as an argument
    cmd.arg(format!("-p{pass}"))
        .arg(program)
        // The ssh client will try keys until it finds one that works.
        // If it tries to many keys that fail it will be disconnected by the server.
        .args(["-o", "PubkeyAuthentication=no"]);
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

trait RunWith {
    fn run_with_captured_stdout(self) -> anyhow::Result<String>;
    fn run_with_logged_stdout(self) -> anyhow::Result<()>;
    fn run_with_inherited_stdout(self) -> anyhow::Result<()>;
}

impl RunWith for std::process::Command {
    fn run_with_captured_stdout(mut self) -> anyhow::Result<String> {
        self.stdout(std::process::Stdio::piped());
        debug!("Spawning child {self:#?}...");
        let mut child = self.spawn()?;
        let mut stdout = child.stdout.take().unwrap();
        debug!("Waiting for child...");
        let status = child.wait()?;
        if !status.success() {
            anyhow::bail!("Child failed: {status}");
        }
        let mut decoded = String::new();
        stdout.read_to_string(&mut decoded)?;
        Ok(decoded)
    }

    fn run_with_logged_stdout(mut self: std::process::Command) -> anyhow::Result<()> {
        self.stdout(std::process::Stdio::piped());
        debug!("Spawning child {self:#?}...");
        let mut child = self.spawn()?;
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
            anyhow::bail!("Child failed: {status}");
        }
        Ok(())
    }

    fn run_with_inherited_stdout(mut self: std::process::Command) -> anyhow::Result<()> {
        debug!("Running command {self:#?}...");
        let status = self.status()?;
        if !status.success() {
            anyhow::bail!("Child failed: {status}");
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
pub fn run_as_self(
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

/// Run ACAP app on device in a realistic manner.
///
/// `user` and `pass` are the credentials to use for the ssh connection.
/// `host` is the device to connect to.
/// `package` is the name of the ACAP app to emulate.
/// `env` is a map of environment variables to override on the remote host.
///
/// The function assumes that the user has already
/// - enabled SSH on the device,
/// - configured the SSH user with a password and the necessary permissions, and
/// - stopped the app.
pub fn run_as_package(
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

    let mut exec_as_package = std::process::Command::new("su");
    exec_as_package.args(["--shell", "/bin/sh"]);
    exec_as_package.args(["--command", &format!("{exec:?}")]);
    exec_as_package.args([format!("acap-{package}")]);

    // TODO: Consider giving user control over what happens with stdout when running concurrently.
    // The escaping of quotation marks is ridiculous, but it's automatic and empirically verifiable.
    let mut ssh_exec_as_package = ssh(user, pass, host);
    ssh_exec_as_package.arg(format!("{cd:?} && {exec_as_package:?}"));
    ssh_exec_as_package.run_with_inherited_stdout()?;

    // TODO: Consider cleaning up by restoring the original file
    // So far I'm not doing this because
    // 1. I'm lazy,
    // 2. I'm concerned about space,
    // 3. I'm concerned about robustness.
    Ok(())
}

/// Update ACAP app on device without installing it
///
/// - `current_dir` is the absolute dir assumed to prefix relative `paths`
/// - `paths` is a mapping `dst -> src` specifying how to update the app.
/// - `user` and `pass` are the credentials to use for the ssh connection.
/// - `host` is the device to connect to.
/// - `package` is the name of the ACAP app to patch.
///
/// When specifying `paths`
/// - `dst` must be a relative path where the base is the package dir
/// - `src` can be
///     - an absolute path,
///     - a relative path (the base is assumed to be `current_dir`), or
///     - `None` (the path is assumed to be the same as `dst`, but with `current_dir` as base).
///
/// The function assumes that the user has already
/// - enabled SSH on the device,
/// - configured the SSH user with a password and the necessary permissions, and
/// - stopped the app.
pub fn sync_package(
    current_dir: &Path,
    paths: HashMap<PathBuf, Option<PathBuf>>,
    user: &str,
    pass: &str,
    host: &Host,
    package: &str,
) -> anyhow::Result<HashMap<PathBuf, PathBuf>> {
    let package_dir = PathBuf::from("/usr/local/packages").join(package);
    let mut ar = tar::Builder::new(Vec::new());

    let mut copied = HashMap::new();
    for (to, from) in paths {
        assert!(to.is_relative());
        let from_abs = if let Some(from) = from {
            if from.is_relative() {
                current_dir.join(from)
            } else {
                from
            }
        } else {
            current_dir.join(&to)
        };

        if from_abs.is_dir() {
            debug!("Appending dir {from_abs:?}");
            ar.append_dir_all(&to, &from_abs)?;
        } else {
            debug!("Appending reg {from_abs:?}");
            ar.append_file(&to, &mut std::fs::File::open(&from_abs)?)?;
        }
        copied.insert(to, from_abs);
    }
    // TODO: Copy only files that have a more recent `mtime` locally
    let mut ssh_tar = ssh(user, pass, host);
    ssh_tar.args(["tar", "-xvC", package_dir.to_str().unwrap()]);

    ssh_tar.stdin(Stdio::piped());
    ssh_tar.stdout(Stdio::piped());
    debug!("Spawning {ssh_tar:#?}");
    let mut child = ssh_tar.spawn()?;

    child.stdin.take().unwrap().write_all(&ar.into_inner()?)?;

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
    Ok(copied)
}
