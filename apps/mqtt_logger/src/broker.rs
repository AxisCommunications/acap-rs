//! Customized broker with mostly secure defaults.
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    io,
    io::{Read, Write},
    process::Command,
};

use log::info;
use serde::{Deserialize, Serialize};

use crate::hush::Secret;

#[derive(Debug)]
struct HashedPassword(String);

impl Display for HashedPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Return the hashed password for use in a passwd file.
///
/// As implemented this uses the mosquitto_passwd program and is not deterministic.
fn crypt(password: &Secret) -> HashedPassword {
    let mut passwordfile = tempfile::NamedTempFile::new().unwrap();
    let output = Command::new("mosquitto_passwd")
        .args([
            "-b",
            passwordfile.path().to_str().unwrap(),
            "x",
            password.revealed(),
        ])
        .output()
        .unwrap();
    if !output.status.success() {
        panic!("mosquitto_passwd failed with {}", &output.status,);
    }
    let mut buf = String::new();
    passwordfile.read_to_string(&mut buf).unwrap();
    HashedPassword(buf.trim().split_once(':').unwrap().1.to_string())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Options {
    pub port: u16,
    pub certificate: String,
    pub private_key: Secret,
    // TODO: Do not save client passwords in plaintext on server
    pub credentials: HashMap<String, Secret>,
}

// TODO: Allow files to be written to a less temporary location
// The tempfile docs warns that tempfiles could be removed.
// It may also be nice to have the files in a known location for debugging.
pub struct Broker {
    _conf: tempfile::NamedTempFile,
    _cert: tempfile::NamedTempFile,
    _key: tempfile::NamedTempFile,
    _passwd: tempfile::NamedTempFile,
    child: std::process::Child,
}

impl Broker {
    pub fn new(options: &Options) -> anyhow::Result<Self> {
        // I had some problems with mosquitto not finding some files so now all of them are flushed.

        let mut cert = tempfile::NamedTempFile::new()?;
        writeln!(cert, "-----BEGIN CERTIFICATE-----")?;
        writeln!(cert, "{}", options.certificate)?;
        writeln!(cert, "-----END CERTIFICATE-----")?;
        cert.flush()?;

        let mut key = tempfile::NamedTempFile::new()?;
        writeln!(key, "-----BEGIN PRIVATE KEY-----")?;
        writeln!(key, "{}", options.private_key.revealed())?;
        writeln!(key, "-----END PRIVATE KEY-----")?;
        writeln!(key, "-----BEGIN CERTIFICATE-----")?;
        writeln!(key, "{}", options.certificate)?;
        writeln!(key, "-----END CERTIFICATE-----")?;
        key.flush()?;

        let mut passwd = tempfile::NamedTempFile::new()?;
        for (user, pass) in &options.credentials {
            writeln!(passwd, "{}:{}", user, crypt(pass))?;
        }
        passwd.flush()?;

        let mut conf = tempfile::NamedTempFile::new()?;
        writeln!(conf, "password_file {}", passwd.path().to_str().unwrap())?;
        // TODO: Implement proper security.
        writeln!(conf, "listener {}", options.port)?;
        writeln!(conf, "cafile {}", cert.path().to_str().unwrap())?;
        writeln!(conf, "keyfile {}", key.path().to_str().unwrap())?;
        writeln!(conf, "certfile {}", cert.path().to_str().unwrap())?;
        conf.flush()?;

        let child = Command::new("/usr/sbin/mosquitto")
            .args(["-c", conf.path().to_str().unwrap()])
            .spawn()?;

        info!("Broker config: {}", conf.path().to_str().unwrap());
        Ok(Self {
            _conf: conf,
            _cert: cert,
            _key: key,
            _passwd: passwd,
            child,
        })
    }

    pub fn is_running(&mut self) -> io::Result<bool> {
        Ok(self.child.try_wait()?.is_none())
    }
}

impl Drop for Broker {
    fn drop(&mut self) {
        info!("Killing child process");
        self.child.kill().unwrap();
        self.child.wait().unwrap();
    }
}
