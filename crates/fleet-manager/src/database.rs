use std::{collections::HashMap, path::PathBuf, str::FromStr};

use anyhow::{bail, Context};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use tracing::debug;
use url::Host;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub user: String,
    pub pass: String,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, ValueEnum, Deserialize, Serialize)]
pub enum ArchAbi {
    Aarch64,
    Armv7hf,
}

impl FromStr for ArchAbi {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "aarch64" => Ok(Self::Aarch64),
            "armv7hf" => Ok(Self::Armv7hf),
            _ => Err(anyhow::anyhow!("Unrecognized variant {s}")),
        }
    }
}

impl ArchAbi {
    pub fn nickname(&self) -> &'static str {
        match self {
            Self::Aarch64 => "aarch64",
            Self::Armv7hf => "armv7hf",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Device {
    pub host: Host,
    pub http_port: Option<u16>,
    pub https_port: Option<u16>,
    pub arch: ArchAbi,
    pub primary: Account,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatabaseContent {
    version: usize,
    pub devices: HashMap<String, Device>,
}

#[derive(Debug)]
pub struct Database {
    file: PathBuf,
    pub content: DatabaseContent,
}

impl Database {
    pub fn create(file: PathBuf) -> anyhow::Result<Self> {
        debug!("Creating empty database");
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).context(dir.to_string_lossy().to_string())?;
        }
        let database = Self {
            file,
            content: DatabaseContent {
                version: 1,
                devices: HashMap::new(),
            },
        };
        database.save()?;
        Ok(database)
    }

    pub fn open_or_create(file: PathBuf) -> anyhow::Result<Self> {
        if file.exists() {
            debug!("Reading database from {file:?}");
            let data = std::fs::read_to_string(&file).context(format!("{file:?}"))?;
            let content = serde_json::from_str(&data)?;
            Ok(Self { file, content })
        } else {
            Self::create(file)
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let text = serde_json::to_string_pretty(&self.content)?;
        debug!("Writing database to {:?}", self.file);
        std::fs::write(&self.file, text).context(self.file.to_string_lossy().to_string())?;
        Ok(())
    }

    pub fn filtered_aliases(&self, pattern: &str) -> anyhow::Result<Vec<String>> {
        if self.content.devices.is_empty() {
            bail!("No devices have been adopted.")
        }
        let pattern = glob::Pattern::new(pattern)?;
        let aliases: Vec<_> = self
            .content
            .devices
            .keys()
            .filter(|a| pattern.matches(a))
            .cloned()
            .collect();
        if aliases.is_empty() {
            bail!("No devices match the pattern {pattern}");
        }
        Ok(aliases)
    }
}
