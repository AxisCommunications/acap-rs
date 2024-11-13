use crate::manifest::Manifest;
use anyhow::Context;
use std::fmt::{Display, Formatter};

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
    pub(crate) fn from_manifest(manifest: &Manifest) -> anyhow::Result<Option<Self>> {
        let Some(param_config) = manifest.param_config() else {
            return Ok(None);
        };

        let mut entries = Vec::new();
        for obj in param_config.iter() {
            let obj = obj
                .as_object()
                .context("paramConfig element is not an object")?;
            let name = obj
                .get("name")
                .context("paramConfig object has no field key")?
                .as_str()
                .context("paramConfig field name is not a string")?
                .to_string();
            let default = obj
                .get("default")
                .context("paramConfig object has no field default")?
                .as_str()
                .context("paramConfig field default is not a string")?
                .to_string();
            let kind = obj
                .get("type")
                .context("paramConfig object has no field kind")?
                .as_str()
                .context("paramConfig field kind is not a string")?
                .to_string();

            entries.push(match kind.is_empty() {
                false => Entry::Typed {
                    name,
                    default,
                    kind,
                },
                true => Entry::Untyped { name, default },
            })
        }
        Ok(Some(Self(entries)))
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
