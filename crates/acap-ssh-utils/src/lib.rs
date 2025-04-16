#![forbid(unsafe_code)]
mod acap;

use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use flate2::read::GzDecoder;
use log::debug;
use tar::Archive;

use ssh2::{FileStat, Session};

use crate::acap::Manifest;

struct RemoteCommand {
    cmd: String,
}

impl RemoteCommand {
    pub fn new(
        user: Option<impl AsRef<str>>,
        env: Option<&[(impl AsRef<str>, impl AsRef<str>)]>,
        executable: &str,
        args: Option<&[&str]>,
    ) -> Self {
        let mut cmd = if let Some(user) = user {
            let mut cmd = std::process::Command::new("su");
            cmd.arg(user.as_ref());
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
            cmd.arg("--"); // They should be passed to the process, not to 'su'
            cmd.args(args);
        }

        Self {
            cmd: format!("{cmd:?}"),
        }
    }

    pub fn exec(&self, session: &Session) -> Result<(), anyhow::Error> {
        let mut channel = session.channel_session()?;
        channel.handle_extended_data(ssh2::ExtendedData::Merge)?;

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

        channel.wait_eof()?;
        channel.wait_close()?;
        let code = channel.exit_status()?;

        if code != 0 {
            bail!("{} exited with status {}", self.cmd, code);
        }

        Ok(())
    }

    pub fn exec_capture_stdout(&self, session: &Session) -> Result<Vec<u8>, anyhow::Error> {
        let mut channel = session.channel_session()?;
        channel.handle_extended_data(ssh2::ExtendedData::Merge)?;

        channel.exec(&self.cmd)?;
        let mut stdout = channel.stream(0);
        let mut output = Vec::new();
        stdout.read_to_end(&mut output)?;

        channel.wait_eof()?;
        channel.wait_close()?;
        let code = channel.exit_status()?;

        if code != 0 {
            bail!("{} exited with status {}", self.cmd, code);
        }

        Ok(output)
    }
}

/// Run executable on device
///
/// `session` is a ssh2::Session connected to the remote host.
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
    let tmp = RemoteCommand::new(None::<&str>, None::<&[(&str, &str)]>, "mktemp -u", None)
        .exec_capture_stdout(session)?;

    // The output from `mktemp -u` contains a trailing '\n'
    let path = std::str::from_utf8(&tmp)?.strip_suffix('\n').unwrap();

    {
        let path = Path::new(&path);

        let sftp = session.sftp().context("Creating sftp session")?;
        sftp.create(path)
            .context(format!("Creating {:?}", path))?
            .write_all(&std::fs::read(prog)?)
            .context(format!("Writing {:?}", prog))?;
        let mut stat = sftp
            .stat(path)
            .context(format!("Running `stat` on {:?}", path))?;
        // `sftp.create` creates a new file with write-only permissions,
        // but since we expect to run this program we need to mark it executable
        // for the user
        stat.perm = Some(0o100744);
        sftp.setstat(path, stat)
            .context(format!("Updating stat on {:?}", path))?;
    }

    RemoteCommand::new(None::<&str>, Some(env), path, Some(args)).exec(session)
}

// TODO: Consider abstracting away the difference between devices that support developer mode, and
//  those that don't.
/// Run ACAP app on device in a realistic manner.
///
/// `session` is a ssh2::Session connected to the remote host.
/// `package` is the name of the ACAP app to emulate.
/// `env` is a map of environment variables to override on the remote host.
/// `as_root` is a boolean that indicates if the process should be run as root
///     or as the package-user.
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
    as_package_user: bool,
) -> anyhow::Result<()> {
    let cmd = RemoteCommand::new(
        as_package_user.then(|| format!("acap-{package}")),
        Some(env),
        &format!("/usr/local/packages/{package}/{package}"),
        Some(args),
    );

    cmd.exec(session)
}

/// Update ACAP app on device without installing it
///
/// - `package` the location of the `.eap` to upload.
/// - `session` is a ssh2::Session connected to the remote host.
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

    let app_name = if let Some(entry) = entries.by_ref().find(|e| {
        e.as_ref()
            .is_ok_and(|entry| entry.path().unwrap_or_default() == Path::new("manifest.json"))
    }) {
        let mut manifest = String::new();
        entry?.read_to_string(&mut manifest)?;
        let manifest: Manifest = serde_json::from_str(&manifest)?;
        manifest.acap_package_conf.setup.app_name
    } else {
        bail!("Could not find a manifest with the app name");
    };

    let package_dir = PathBuf::from("/usr/local/packages").join(app_name);
    let sftp = session.sftp().context("Creating sftp session")?;
    if sftp.stat(&package_dir).is_err() {
        bail!("Package doesn't exist!");
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

            if let Ok(Some(link)) = entry.link_name() {
                let target = package_dir.join(&entry.path()?);

                // `file_name` fails if the path ends in '..' or is '/', neither of which should
                // be the case for a symlink
                if sftp
                    .readlink(&target)
                    .is_ok_and(|l| l.file_name().unwrap() == link)
                {
                    debug!("Symlink {target:?} -> {link:?} exists, skipping");
                    continue;
                }

                debug!("Adding symlink {target:?} -> {link:?}");
                sftp.symlink(&package_dir.join(&link), &target)
                    .context(format!("Adding symlink {target:?} -> {link:?}"))?;

                continue;
            }

            if header.entry_type().is_dir() {
                // If the directory can't be opened, then it doesn't exist so we need to create it.
                // TODO: What if permissions has changed?
                if sftp.opendir(entry.path()?).is_err() {
                    sftp.mkdir(&header.path()?, header.mode()? as i32)
                        .context(format!("Creating directory {:?}", entry.path()?))?;
                }

                continue;
            }

            entry.read_to_end(&mut buf)?;
            let mut file = sftp
                .create(&package_dir.join(&entry.path()?))
                .context(format!("Creating {:?}", entry.path()?))?;
            file.write_all(&buf)
                .context(format!("Writing to {:?}", entry.path()?))?;
            file.setstat(stat)
                .context(format!("Updating stat on {:?}", entry.path()?))?;
        }
    }

    Ok(())
}
