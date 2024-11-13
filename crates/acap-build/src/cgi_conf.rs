use crate::manifest::Manifest;
use anyhow::Context;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
enum Entry {
    Fast { access: String, name: String },
    Other { access: String, name: String },
}

#[derive(Debug)]
pub(crate) struct CgiConf(Vec<Entry>);

impl CgiConf {
    pub(crate) fn from_manifest(manifest: &Manifest) -> anyhow::Result<Option<Self>> {
        let Some(conf) = manifest.http_config() else {
            return Ok(None);
        };

        let mut entries = Vec::new();
        for obj in conf.iter() {
            let obj = obj
                .as_object()
                .context("httpConfig element is not an object")?;

            let kind = obj
                .get("type")
                .context("httpConfig object has no field kind")?
                .as_str()
                .context("httpConfig field kind is not a string")?;
            if kind == "directory" {
                continue;
            }

            let name = obj
                .get("name")
                .context("httpConfig object has no field name")?
                .as_str()
                .context("httpConfig field name is not a string")?
                .trim_start_matches('/')
                .to_string();

            let access = obj
                .get("access")
                .context("httpConfig object has no field access")?
                .as_str()
                .context("httpConfig field access is not a string")?;

            let access = match access {
                "admin" => "administrator".to_string(),
                access => access.to_string(),
            };
            entries.push(match kind {
                "fastCgi" => Entry::Fast { access, name },
                _ => Entry::Other { access, name },
            })
        }
        Ok(Some(Self(entries)))
    }

    pub(crate) fn file_name() -> &'static str {
        "cgi.conf"
    }
}

impl Display for CgiConf {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for cgi in &self.0 {
            match &cgi {
                Entry::Fast { access, name } => writeln!(f, "{access} /{name} fastCgi")?,
                Entry::Other { access, name } => writeln!(f, "{access} /{name}")?,
            }
        }
        Ok(())
    }
}
