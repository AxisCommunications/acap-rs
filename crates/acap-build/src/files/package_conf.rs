//! Code for populating the `package.conf` file
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use anyhow::{bail, Context};
use log::debug;
use regex::Regex;
use semver::Version;
use serde_json::{Map, Value};

use crate::{files::manifest::Manifest, Architecture};

#[derive(Clone, Debug)]
pub(crate) struct PackageConf(HashMap<&'static str, String>);

impl PackageConf {
    pub(crate) fn new(
        manifest: &Manifest,
        other_files: &[String],
        default_arch: Architecture,
    ) -> anyhow::Result<PackageConf> {
        let mut package_conf = Self(HashMap::new());
        package_conf.set_custom_from_manifest(manifest)?;
        package_conf.set_custom_from_other_files(other_files)?;
        package_conf.set_defaults(default_arch);
        Ok(package_conf)
    }

    fn set_custom_from_manifest(&mut self, manifest: &Manifest) -> anyhow::Result<()> {
        let parameters: HashMap<_, _> = PARAMETERS
            .iter()
            .flat_map(|p| p.source.map(|s| (s, p.name)))
            .collect();

        let flat_manifest = flattened(manifest.as_object()?.clone());
        for (path, value) in flat_manifest {
            match path.as_str() {
                "acapPackageConf.setup.version" => {
                    let v = value
                        .as_str()
                        .context("acapPackageConf.setup.version is not a string")?;
                    let v = Version::parse(v)?;
                    debug_assert_eq!(self.0.insert("APPMAJORVERSION", v.major.to_string()), None);
                    debug_assert_eq!(self.0.insert("APPMINORVERSION", v.minor.to_string()), None);
                    debug_assert_eq!(self.0.insert("APPMICROVERSION", v.patch.to_string()), None);
                }
                "acapPackageConf.setup.vendorUrl" => {
                    let re = Regex::new("(?:(?:http|https)://)?(.+)")
                        .expect("Hard-coded regex is valid");
                    let v = value
                        .as_str()
                        .context("acapPackageConf.setup.vendorUrl is not a string")?;
                    let Some(caps) = re.captures(v) else {
                        bail!("Expected vendor url to match regex {:?}", re)
                    };
                    let domain_name = caps
                        .get(1)
                        .expect("Hard coded regex as exactly one capture group")
                        .as_str();
                    debug_assert_eq!(
                        self.0.insert(
                            "VENDORHOMEPAGELINK",
                            format!(r#"<a href="{v}" target="_blank">{domain_name}</a>"#),
                        ),
                        None
                    );
                }
                "acapPackageConf.configuration.httpConfig" => {
                    let v = value
                        .as_array()
                        .context("acapPackageConf.configuration.httpConfig is not an array")?;
                    if !v.is_empty() {
                        debug_assert_eq!(
                            self.0.insert("HTTPCGIPATHS", "cgi.conf".to_string()),
                            None
                        )
                    }
                }
                path => {
                    if let Some(name) = parameters.get(path) {
                        let v = value
                            .as_str()
                            .with_context(|| format!("{path} is not a string"))?
                            .to_string();
                        debug_assert_eq!(self.0.insert(name, v), None);
                    } else {
                        debug!("{path} skipped, no corresponding parameter in package.conf")
                    }
                }
            }
        }
        Ok(())
    }

    fn set_custom_from_other_files(&mut self, other_files: &[String]) -> anyhow::Result<()> {
        if !other_files.is_empty() {
            debug_assert_eq!(self.0.insert("OTHERFILES", other_files.join(" ")), None);
        }
        Ok(())
    }

    fn set_defaults(&mut self, arch: Architecture) {
        // If not set from the manifest, eap-create.sh would try to infer it.
        self.0
            .entry("APPTYPE")
            .or_insert(arch.nickname().to_string());

        // If not set from the manifest, eap-create.sh would try to infer it from the exe.
        // But we know that it must be set for the manifest to be valid.
        // TODO: Fail explicitly if app name is not set.
        let app_name = self.0.get("APPNAME").cloned().unwrap_or_default();
        // If not set from the manifest, eap-create.sh would fall back on the app name
        self.0.entry("PACKAGENAME").or_insert(app_name);

        for Parameter { name, default, .. } in PARAMETERS {
            if let Some(v) = default {
                self.0.entry(name).or_insert(v.to_string());
            }
        }
    }
}

impl Display for PackageConf {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for Parameter { name, .. } in PARAMETERS {
            if let Some(value) = self.0.get(name) {
                if name == "VENDORHOMEPAGELINK" {
                    writeln!(f, r#"{name}='{value}'"#)?;
                } else {
                    writeln!(f, r#"{name}="{value}""#)?;
                }
            }
        }
        Ok(())
    }
}

struct Parameter {
    name: &'static str,
    source: Option<&'static str>,
    default: Option<&'static str>,
}

// TODO: Consider generating this (semi-) automatically from its sources:
// - conversion.py
// - package-conf-parameters.cfg
const PARAMETERS: [Parameter; 25] = [
    Parameter {
        name: "PACKAGENAME",
        source: Some("acapPackageConf.setup.friendlyName"),
        default: Some(""),
    },
    // Not supported. The name of the ACAP will be taken from PACKAGENAME instead
    Parameter {
        name: "MENUNAME",
        source: None,
        default: None,
    },
    Parameter {
        name: "APPTYPE",
        source: Some("acapPackageConf.setup.architecture"),
        default: Some(""),
    },
    Parameter {
        name: "APPNAME",
        source: Some("acapPackageConf.setup.appName"),
        default: Some(""),
    },
    Parameter {
        name: "APPID",
        source: Some("acapPackageConf.setup.appId"),
        default: Some(""),
    },
    // Not supported by manifest ACAP applications
    Parameter {
        name: "LICENSENAME",
        source: None,
        default: Some("Available"),
    },
    Parameter {
        name: "LICENSEPAGE",
        source: Some("acapPackageConf.copyProtection.method"),
        default: Some("none"),
    },
    // If the application uses a custom licensing solution but still want to be able to use the
    // standard list.cgi and WebUI to display license status.
    // Use this option to allow the application to hook into the AXIS API.
    Parameter {
        name: "LICENSE_CHECK_ARGS",
        source: Some("acapPackageConf.copyProtection.customOptions"),
        default: None,
    },
    Parameter {
        name: "VENDOR",
        source: Some("acapPackageConf.setup.vendor"),
        default: Some("-"),
    },
    Parameter {
        name: "REQEMBDEVVERSION",
        source: Some("acapPackageConf.setup.embeddedSdkVersion"),
        default: Some("2.0"),
    },
    Parameter {
        name: "APPMAJORVERSION",
        source: None, // Treated specially
        default: Some("1"),
    },
    Parameter {
        name: "APPMINORVERSION",
        source: None, // Treated specially
        default: Some("0"),
    },
    Parameter {
        name: "APPMICROVERSION",
        source: None, // Treated specially
        default: Some("0"),
    },
    Parameter {
        name: "APPGRP",
        source: Some("acapPackageConf.setup.user.group"),
        default: Some("sdk"),
    },
    Parameter {
        name: "APPUSR",
        source: Some("acapPackageConf.setup.user.username"),
        default: Some("sdk"),
    },
    Parameter {
        name: "APPOPTS",
        source: Some("acapPackageConf.setup.runOptions"),
        default: Some(""),
    },
    Parameter {
        name: "OTHERFILES",
        source: None,
        default: Some(""),
    },
    Parameter {
        name: "SETTINGSPAGEFILE",
        source: Some("acapPackageConf.configuration.settingPage"),
        default: Some(""),
    },
    // Special name on the link to the settings page is not supported
    Parameter {
        name: "SETTINGSPAGETEXT",
        source: None,
        default: Some(""),
    },
    Parameter {
        name: "VENDORHOMEPAGELINK",
        source: None, // Treated specially
        default: Some(""),
    },
    // Pre-upgrade scripts are not supported
    Parameter {
        name: "PREUPGRADESCRIPT",
        source: None,
        default: Some(""),
    },
    Parameter {
        name: "POSTINSTALLSCRIPT",
        source: Some("acapPackageConf.installation.postInstallScript"),
        default: Some(""),
    },
    Parameter {
        name: "STARTMODE",
        source: Some("acapPackageConf.setup.runMode"),
        default: Some("never"),
    },
    Parameter {
        name: "HTTPCGIPATHS",
        source: None, // Treated specially
        default: Some(""),
    },
    // Only supported for pre-installed applications
    Parameter {
        name: "AUTOSTART",
        source: None,
        default: None,
    },
];

fn flattened(root: Map<String, Value>) -> Vec<(String, Value)> {
    let mut output = Vec::new();
    let mut values = vec![(String::new(), Value::Object(root))];
    while let Some((path, value)) = values.pop() {
        match value {
            Value::Null => output.push((path, Value::Null)),
            Value::Bool(b) => output.push((path, Value::Bool(b))),
            Value::Number(n) => output.push((path, Value::Number(n))),
            Value::String(s) => output.push((path, Value::String(s.clone()))),
            Value::Array(a) => output.push((path, Value::Array(a.clone()))),
            Value::Object(o) => {
                for (k, v) in o.into_iter().rev() {
                    debug_assert!(!k.contains('.'));
                    let mut p = path.clone();
                    if !p.is_empty() {
                        p.push('.');
                    }
                    p.push_str(&k.to_string());
                    values.push((p, v));
                }
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    #[test]
    fn stringify_works_on_example() {
        let value = json!(
            {
                "a": 1,
                "b": [2, 3],
                "c": [{"i":4}],
                "d": {"j": 5},
                "e": {
                    "k": 6,
                    "l": {"x": 7},
                    "m": 8,
                },
            }
        );
        let Value::Object(object) = value else {
            panic!("expected object");
        };
        let actual = flattened(object);
        let expected = vec![
            ("a".to_string(), json!(1)),
            ("b".to_string(), json!([2, 3])),
            ("c".to_string(), json!([{"i": 4}])),
            ("d.j".to_string(), json!(5)),
            ("e.k".to_string(), json!(6)),
            ("e.l.x".to_string(), json!(7)),
            ("e.m".to_string(), json!(8)),
        ];
        assert_eq!(actual, expected);
    }
}
