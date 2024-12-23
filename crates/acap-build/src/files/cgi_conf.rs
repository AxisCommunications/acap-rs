//! Code for populating the `cgi.conf` file
use std::fmt::{Display, Formatter};

use log::debug;

use crate::{
    files::manifest::Manifest,
    json_ext,
    json_ext::{MapExt, ValueExt},
};

#[derive(Debug)]
enum Entry {
    Fast { access: String, name: String },
    Other { access: String, name: String },
}

#[derive(Debug)]
pub(crate) struct CgiConf(Vec<Entry>);

impl CgiConf {
    pub(crate) fn new(manifest: &Manifest) -> anyhow::Result<Option<Self>> {
        let conf = match manifest.try_find_http_config() {
            Ok(v) => v,
            Err(json_ext::Error::KeyNotFound(_)) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let mut entries = Vec::new();
        for obj in conf.iter() {
            let obj = obj.try_to_object()?;

            let kind = obj.try_get_str("type")?;
            if kind == "directory" {
                debug!("Skipping httpConfig of type directory");
                continue;
            }

            let name = obj.try_get_str("name")?.trim_start_matches('/').to_string();

            let access = match obj.try_get_str("access")? {
                "admin" => "administrator",
                access => access,
            }
            .to_string();

            entries.push(match kind {
                "fastCgi" => Entry::Fast { access, name },
                _ => Entry::Other { access, name },
            })
        }
        Ok(Some(Self(entries)))
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
