#![forbid(unsafe_code)]
mod acap;

use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::bail;
use flate2::read::GzDecoder;
use tar::Archive;

use ssh2::{Channel, FileStat, Session};

use crate::acap::Manifest;

struct RemoteCommand {
    cmd: String,
}

impl RemoteCommand {
    pub fn new(
        user: Option<&str>,
        env: Option<&[(impl AsRef<str>, impl AsRef<str>)]>,
        executable: &str,
        args: Option<&[&str]>,
    ) -> Self {
        let mut cmd = if let Some(user) = user {
            let mut cmd = std::process::Command::new("su");
            cmd.arg(user);
            cmd
        } else {
            std::process::Command::new("sh")
        };

        cmd.arg("-c");

        if let Some(env) = env {
            cmd.envs(env.iter().map(|(k, v)| (k.as_ref(), v.as_ref())));
        }
        cmd.env("G_SLICE", "always-malloc");

        cmd.arg(executable);
        if let Some(args) = args {
            cmd.args(args);
        }

        Self {
            cmd: format!("{cmd:?}"),
        }
    }

    pub fn exec(&self, channel: &mut Channel) -> Result<(), anyhow::Error> {
        channel.exec(&self.cmd)?;
        let mut stdout = channel.stream(0);
        let mut buf = [0; 4096];
        loop {
            let n = stdout.read(&mut buf)?;
            if n == 0 {
                break;
            }
            print!("{}", std::str::from_utf8(&buf[..n])?);
            stdout.flush()?;
        }

        Ok(())
    }

    pub fn exec_capture_stdout(&self, channel: &mut Channel) -> Result<String, anyhow::Error> {
        channel.exec(&self.cmd)?;
        let mut stdout = channel.stream(0);
        let mut buf = [0; 4096];
        let n = stdout.read(&mut buf)?;
        Ok(std::str::from_utf8(&buf[..n])?.to_string())
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
pub fn run_other<S: AsRef<str>>(
    prog: &Path,
    session: &Session,
    env: &[(S, S)],
    args: &[&str],
) -> anyhow::Result<()> {
    let sftp = session.sftp()?;
    let mut channel = session.channel_session()?;

    let tmp = RemoteCommand::new(None, None::<&[(&str, &str)]>, "mktemp", None)
        .exec_capture_stdout(&mut channel)?;

    sftp.create(Path::new(&tmp))?
        .write_all(&std::fs::read(prog)?)?;

    RemoteCommand::new(None, Some(env), &tmp, Some(args)).exec(&mut channel)
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
pub fn run_package<S: AsRef<str>>(
    session: &Session,
    package: &str,
    env: &[(S, S)],
    args: &[&str],
) -> anyhow::Result<()> {
    let mut channel = session.channel_session()?;
    let cmd = RemoteCommand::new(
        Some(&format!("acap-{package}")),
        Some(env),
        &format!("/usr/local/packages/{package}/{package}"),
        Some(args),
    );
    channel.handle_extended_data(ssh2::ExtendedData::Merge)?;

    cmd.exec(&mut channel)
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
pub fn patch_package(package: &Path, session: &Session) -> anyhow::Result<()> {
    let mut full = Archive::new(GzDecoder::new(File::open(package)?));
    let mut entries = full.entries()?;

    let app_name = if let Some(entry) = entries
        .by_ref()
        .find(|e| e.as_ref().unwrap().path().unwrap_or_default() == Path::new("manifest.json"))
    {
        let mut manifest = String::new();
        entry?.read_to_string(&mut manifest)?;
        let manifest: Manifest = serde_json::from_str(&manifest)?;
        manifest.acap_package_conf.setup.app_name
    } else {
        bail!("Could not find a manifest with the app name");
    };

    let package_dir = PathBuf::from("/usr/local/packages").join(app_name);
    let sftp = session.sftp()?;
    if sftp.stat(&package_dir).is_err() {
        return Err(anyhow::anyhow!("Package doesn't exist"));
    }

    let mut full = Archive::new(GzDecoder::new(File::open(package)?));

    // TODO: Only upload changed files
    for entry in full.entries()? {
        let mut entry = entry?;
        let mut buf = Vec::new();
        let header = entry.header();
        if entry.path()? != Path::new("manifest.json") && entry.path()? != Path::new("package.conf")
        {
            let stat = FileStat {
                gid: Some(header.gid()?.try_into()?),
                uid: Some(header.uid()?.try_into()?),
                perm: Some(header.mode()?),
                atime: None,
                mtime: Some(header.mtime()?),
                size: None,
            };
            _ = entry.read_to_end(&mut buf)?;
            println!("Writing file: {:?}", entry.path()?);
            let mut file = sftp.create(&package_dir.join(&entry.path()?))?;
            file.write_all(&buf)?;
            file.setstat(stat)?;
        }
    }

    Ok(())
}
