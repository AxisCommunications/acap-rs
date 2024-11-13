use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest(pub(crate) Map<String, Value>);

impl Manifest {
    // TODO: Consider returning an error if any value is of the wrong type.

    pub fn app_name(&self) -> Option<&str> {
        self.0
            .get("acapPackageConf")?
            .as_object()?
            .get("setup")?
            .as_object()?
            .get("appName")?
            .as_str()
    }
    pub(crate) fn architecture(&self) -> Option<&str> {
        self.0
            .get("acapPackageConf")?
            .as_object()?
            .get("setup")?
            .as_object()?
            .get("architecture")?
            .as_str()
    }

    pub(crate) fn http_config(&self) -> Option<&Vec<Value>> {
        self.0
            .get("acapPackageConf")?
            .as_object()?
            .get("configuration")?
            .as_object()?
            .get("httpConfig")?
            .as_array()
    }

    pub(crate) fn param_config(&self) -> Option<&Vec<Value>> {
        self.0
            .get("acapPackageConf")?
            .as_object()?
            .get("configuration")?
            .as_object()?
            .get("paramConfig")?
            .as_array()
    }

    pub(crate) fn pre_uninstall_script(&self) -> Option<&Value> {
        self.0
            .get("acapPackageConf")?
            .as_object()?
            .get("uninstallation")?
            .as_object()?
            .get("preUninstallScript")
    }

    pub(crate) fn version(&self) -> Option<&str> {
        self.0
            .get("acapPackageConf")?
            .as_object()?
            .get("setup")?
            .as_object()?
            .get("version")?
            .as_str()
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
