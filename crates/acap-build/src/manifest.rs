use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{json_ext, json_ext::MapExt};

#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest(pub(crate) Map<String, Value>);

impl Manifest {
    pub fn find_app_name(&self) -> anyhow::Result<&str> {
        Ok(self.try_find_app_name()?)
    }

    pub(crate) fn try_find_app_name(&self) -> json_ext::Result<&str> {
        self.0
            .try_get_object("acapPackageConf")?
            .try_get_object("setup")?
            .try_get_str("appName")
    }

    pub(crate) fn try_find_architecture(&self) -> json_ext::Result<&str> {
        self.0
            .try_get_object("acapPackageConf")?
            .try_get_object("setup")?
            .try_get_str("architecture")
    }

    pub(crate) fn try_find_http_config(&self) -> json_ext::Result<&Vec<Value>> {
        self.0
            .try_get_object("acapPackageConf")?
            .try_get_object("configuration")?
            .try_get_array("httpConfig")
    }

    pub(crate) fn try_find_param_config(&self) -> json_ext::Result<&Vec<Value>> {
        self.0
            .try_get_object("acapPackageConf")?
            .try_get_object("configuration")?
            .try_get_array("paramConfig")
    }

    pub(crate) fn try_find_pre_uninstall_script(&self) -> json_ext::Result<&str> {
        self.0
            .try_get_object("acapPackageConf")?
            .try_get_object("uninstallation")?
            .try_get_str("preUninstallScript")
    }

    pub(crate) fn try_find_version(&self) -> json_ext::Result<&str> {
        self.0
            .try_get_object("acapPackageConf")?
            .try_get_object("setup")?
            .try_get_str("version")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_deserialize() {
        let manifest_paths = glob::glob("../../apps/*/manifest.json")
            .unwrap()
            .collect::<Vec<_>>();
        assert!(!manifest_paths.is_empty());

        for manifest_path in manifest_paths {
            println!("{manifest_path:?}");
            let input = std::fs::read_to_string(manifest_path.unwrap()).unwrap();
            let _: Manifest = serde_json::from_str(&input).unwrap();
            // It should not be hard to assert that serializing the manifest gives an output that
            // is equivalent to the input.
            // TODO: Consider testing serialization
        }
    }
}
