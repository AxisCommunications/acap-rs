use std::fmt::{Display, Formatter};

use crate::manifest::Manifest;

#[derive(Debug)]
enum Entry {
    Fast { access: String, name: String },
    Other { access: String, name: String },
}

#[derive(Debug)]
pub(crate) struct CgiConf(Vec<Entry>);

impl CgiConf {
    pub(crate) fn from_manifest(manifest: &Manifest) -> Result<Self, CgiConfError> {
        let Some(configuration) = manifest.acap_package_conf.configuration.as_ref() else {
            return Err(CgiConfError::NoHttpConfig);
        };
        let Some(conf) = configuration.http_config.as_ref() else {
            return Err(CgiConfError::NoHttpConfig);
        };

        let mut entries = Vec::new();
        for obj in conf.iter() {
            let kind = obj.kind.as_str();
            if kind == "directory" {
                continue;
            }

            let Some(name) = obj.name.as_ref() else {
                return Err(CgiConfError::BadManifest);
            };
            let name = name.trim_start_matches('/').to_string();

            let access = match obj.access.as_ref() {
                "admin" => "administrator".to_string(),
                access => access.to_string(),
            };

            entries.push(match kind {
                "fastCgi" => Entry::Fast { access, name },
                _ => Entry::Other { access, name },
            })
        }
        Ok(Self(entries))
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

#[derive(Debug)]
pub(crate) enum CgiConfError {
    NoHttpConfig,
    BadManifest,
}
