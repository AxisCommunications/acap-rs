use std::fmt::{Display, Formatter};

use crate::manifest::Manifest;

#[derive(Debug)]
enum Entry {
    Typed {
        name: String,
        default: String,
        kind: String,
    },
    Untyped {
        name: String,
        default: String,
    },
}

#[derive(Debug)]
pub(crate) struct ParamConf(Vec<Entry>);

impl ParamConf {
    pub(crate) fn from_manifest(manifest: &Manifest) -> Result<Self, &'static str> {
        let Some(configuration) = manifest.acap_package_conf.configuration.as_ref() else {
            return Err("no configuration in manifest");
        };
        let Some(param_config) = configuration.param_config.as_ref() else {
            return Err("no paramConfig in manifest");
        };

        let mut entries = Vec::new();
        for obj in param_config.iter() {
            let name = obj.name.to_string();
            let default = obj.default.to_string();
            entries.push(match obj.kind.is_empty() {
                false => Entry::Typed {
                    name,
                    default,
                    kind: obj.kind.to_string(),
                },
                true => Entry::Untyped { name, default },
            })
        }
        Ok(Self(entries))
    }

    pub(crate) fn file_name() -> &'static str {
        "param.conf"
    }
}

impl Display for ParamConf {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for param in &self.0 {
            match param {
                Entry::Typed {
                    name,
                    default,
                    kind,
                } => writeln!(f, r#"{name}="{default}" type="{kind}""#)?,
                Entry::Untyped { name, default } => writeln!(f, "{name}={default}")?,
            }
        }
        Ok(())
    }
}
