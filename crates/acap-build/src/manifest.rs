use crate::{
    json_ext,
    json_ext::{MapExt, ValueExt},
    Architecture,
};
use anyhow::{bail, Context};
use log::debug;
use serde::Serialize;
use serde_json::ser::PrettyFormatter;
use serde_json::{Map, Serializer, Value};
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::BufWriter;

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
            let setup = manifest
                .0
                .get_mut("acapPackageConf")
                .context("no key acapPackageConf in manifest")?
                .get_mut("setup")
                .context("no key setup in acapPackageConf")?
                .as_object_mut()
                .context("Expected setup to be object")?;
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

    pub(crate) fn as_value(&self) -> &Value {
        &self.0
    }

    pub(crate) fn as_object(&self) -> json_ext::Result<&Map<String, Value>> {
        self.0.try_to_object()
    }

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
}

impl Display for Manifest {

    // TODO: Consider refactoring this to avoid the intermediate string
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // This file is included in the EAP, so for as long as we want bit-exact output, we must
        // take care to serialize the manifest the same way as the python implementation.
        // let mut writer = BufWriter::new(String::new());
        let mut data = Vec::new();
        let mut serializer =
            Serializer::with_formatter(&mut data, PrettyFormatter::with_indent(b"    "));
        self.0.serialize(&mut serializer).unwrap();
        f.write_str(String::from_utf8(data).unwrap().as_str())?;
        Ok(())
    }
}
