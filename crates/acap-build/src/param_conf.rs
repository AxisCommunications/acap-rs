//! Code for populating the `param.conf` file
use std::fmt::{Display, Formatter};

use crate::{
    json_ext,
    json_ext::{MapExt, ValueExt},
    manifest::Manifest,
};

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
    pub(crate) fn new(manifest: &Manifest) -> anyhow::Result<Option<Self>> {
        let param_config = match manifest.try_find_param_config() {
            Ok(v) => v,
            Err(json_ext::Error::KeyNotFound(_)) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let mut entries = Vec::new();
        for obj in param_config.iter() {
            let obj = obj.try_to_object()?;
            let name = obj.try_get_str("name")?.to_string();
            let default = obj.try_get_str("default")?.to_string();
            let kind = obj.try_get_str("type")?.to_string();

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
