use anyhow::bail;
use log::debug;
use serde::Serialize;
use serde_json::{ser::PrettyFormatter, Map, Serializer, Value};

use crate::{
    json_ext,
    json_ext::{MapExt, ValueExt},
    Architecture,
};

#[derive(Debug)]
pub(crate) struct Manifest(Value);

impl Manifest {
    pub(crate) fn new(manifest: Value, architecture: Architecture) -> anyhow::Result<Self> {
        let mut manifest = Self(manifest);
        let mut schema_version = manifest
            .as_object()?
            .try_get_str("schemaVersion")?
            .to_string();

        // Make it valid semver
        for _ in 0..(2 - schema_version.chars().filter(|&c| c == '.').count()) {
            schema_version.push_str(".0");
        }
        let schema_version = semver::Version::parse(&schema_version)?;
        if schema_version > semver::Version::new(1, 3, 0) {
            let setup = manifest.try_find_setup_mut()?;
            if let Some(a) = setup.get("architecture") {
                if a != "all" && a != architecture.nickname() {
                    bail!(
                        "Architecture in manifest ({a}) is not compatible with built target ({:?})",
                        architecture
                    );
                }
            } else {
                debug!(
                    "Architecture not set in manifest, using {:?}",
                    &architecture
                );
                setup.insert(
                    "architecture".to_string(),
                    Value::String(architecture.nickname().to_string()),
                );
            }
        }
        Ok(manifest)
    }

    pub(crate) fn as_object(&self) -> json_ext::Result<&Map<String, Value>> {
        self.0.try_to_object()
    }

    pub(crate) fn as_object_mut(&mut self) -> json_ext::Result<&mut Map<String, Value>> {
        self.0.try_to_object_mut()
    }

    // TODO: Consider generalizing this to something like `try_get_as_str(&self, path: &[&str])`
    pub(crate) fn try_find_app_name(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("setup")?
            .try_get_str("appName")
    }

    pub(crate) fn try_find_architecture(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("setup")?
            .try_get_str("architecture")
    }

    pub(crate) fn try_find_friendly_name(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("setup")?
            .try_get_str("friendlyName")
    }

    pub(crate) fn try_find_http_config(&self) -> json_ext::Result<&Vec<Value>> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("configuration")?
            .try_get_array("httpConfig")
    }

    pub(crate) fn try_find_param_config(&self) -> json_ext::Result<&Vec<Value>> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("configuration")?
            .try_get_array("paramConfig")
    }

    pub(crate) fn try_find_post_install_script(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("uninstallation")?
            .try_get_str("preUninstallScript")
    }

    pub(crate) fn try_find_pre_uninstall_script(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("uninstallation")?
            .try_get_str("preUninstallScript")
    }

    pub(crate) fn try_find_version(&self) -> json_ext::Result<&str> {
        self.as_object()?
            .try_get_object("acapPackageConf")?
            .try_get_object("setup")?
            .try_get_str("version")
    }

    pub(crate) fn try_find_setup_mut(&mut self) -> json_ext::Result<&mut Map<String, Value>> {
        self.as_object_mut()?
            .try_get_object_mut("acapPackageConf")?
            .try_get_object_mut("setup")
    }

    pub(crate) fn try_to_string(&self) -> anyhow::Result<String> {
        // This file is included in the EAP, so for as long as we want bit-exact output, we must
        // take care to serialize the manifest the same way as the python implementation.
        // let mut writer = BufWriter::new(String::new());
        let mut data = Vec::new();
        let mut serializer =
            Serializer::with_formatter(&mut data, PrettyFormatter::with_indent(b"    "));
        self.0.serialize(&mut serializer)?;
        Ok(String::from_utf8(data)?)
    }
}
