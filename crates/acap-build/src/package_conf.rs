use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
};

use anyhow::bail;
use log::debug;
use regex::Regex;
use semver::Version;
use serde_json::Value;

use crate::{json_ext, Architecture};
use crate::manifest::Manifest;

#[derive(Clone, Debug)]
pub(crate) struct PackageConf(HashMap<&'static str, String>);

impl PackageConf {
    pub fn file_name() -> &'static str {
        "package.conf"
    }

    pub fn new(
        manifest: &Manifest,
        outpath: &Path,
        mut otherfiles: Vec<PathBuf>,
        arch: Architecture,
    ) -> anyhow::Result<PackageConf> {
        match manifest.try_find_pre_uninstall_script() {
            Ok(p) => otherfiles.push(PathBuf::from(p)),
            Err(json_ext::Error::KeyNotFound(_)) => {}
            Err(e) => return Err(e.into()),
        }

        let otherfiles = otherfiles
            .iter()
            .map(|f| outpath.join(f))
            .collect::<Vec<_>>();

        let mut package_conf = Self(HashMap::new());
        package_conf.update_from_manifest(manifest)?;
        package_conf.update_from_other_files(&otherfiles, outpath)?;
        package_conf.update_from_app_type(arch);
        Ok(package_conf)
    }

    fn update_from_manifest(&mut self, manifest: &Manifest) -> anyhow::Result<()> {
        let parameters: HashMap<_, _> = PARAMETERS
            .iter()
            .flat_map(|p| p.source.map(|s| (s, p.name)))
            .collect();

        let flat_manifest = stringify(manifest.as_value());
        let mut cgi_parsed = false;
        for (path, value) in flat_manifest {
            match path.as_str() {
                "acapPackageConf.setup.version" => {
                    let Value::String(v) = value else {
                        bail!("acapPackageConf.setup.version is not a string")
                    };
                    let v = Version::parse(&v)?;
                    self.0.insert("APPMAJORVERSION", v.major.to_string());
                    self.0.insert("APPMINORVERSION", v.minor.to_string());
                    self.0.insert("APPMICROVERSION", v.patch.to_string());
                }
                "acapPackageConf.setup.vendorUrl" => {
                    let re = Regex::new("(?:(?:http|https)://)?(.+)")
                        .expect("Hard-coded regex is valid");
                    let Value::String(v) = value else {
                        bail!("acapPackageConf.setup.vendorUrl is not a string")
                    };
                    let Some(caps) = re.captures(&v) else {
                        bail!("Expected vendor url to match regex {:?}", re)
                    };
                    let domain_name = caps
                        .get(1)
                        .expect("Hard coded regex as exactly one capture group")
                        .as_str();
                    self.0.insert(
                        "VENDORHOMEPAGELINK",
                        format!(r#"<a href="{v}" target="_blank">{domain_name}</a>"#),
                    );
                }
                path if path.starts_with("acapPackageConf.configuration.httpConfig") => {
                    if !cgi_parsed {
                        cgi_parsed = true;
                        self.0.insert("HTTPCGIPATHS", "cgi.conf".to_string());
                    }
                }
                path => {
                    if let Some(name) = parameters.get(path) {
                        let Value::String(v) = value else {
                            bail!("{path} is not a string")
                        };
                        self.0.insert(name, v);
                    } else {
                        debug!("{path} skipped, no corresponding parameter in package.conf")
                    }
                }
            }
        }
        Ok(())
    }

    fn update_from_other_files(
        &mut self,
        otherfiles: &[PathBuf],
        outpath: &Path,
    ) -> anyhow::Result<()> {
        if !otherfiles.is_empty() {
            let mut relpaths = Vec::new();
            for file in otherfiles {
                let relpath = match file.is_absolute() {
                    true => file.strip_prefix(outpath)?,
                    false => file,
                };
                relpaths.push(relpath.to_string_lossy().to_string());
            }
            self.0.insert("OTHERFILES", relpaths.join(" "));
        }
        Ok(())
    }

    fn update_from_app_type(&mut self, arch: Architecture) {
        self.0
            .entry("APPTYPE")
            .or_insert(arch.nickname().to_string());

        let app_name = self.0.get("APPNAME").cloned().unwrap_or_default();
        self.0.entry("PACKAGENAME").or_insert(app_name);

        for Parameter { name, default, .. } in PARAMETERS {
            if let Some(v) = default {
                self.0.entry(name).or_insert(v.to_string());
            }
        }
    }
    pub fn http_cig_paths(&self) -> Option<&String> {
        self.0.get("HTTPCGIPATHS")
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
    Parameter {
        name: "MENUNAME",
        source: None,
        default: None, // TODO: Figure out why this should be `None`
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
        source: Some("acapPackageConf.setup.version"),
        default: Some("1"),
    },
    Parameter {
        name: "APPMINORVERSION",
        source: Some("acapPackageConf.setup.version"),
        default: Some("0"),
    },
    Parameter {
        name: "APPMICROVERSION",
        source: Some("acapPackageConf.setup.version"),
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
    Parameter {
        name: "SETTINGSPAGETEXT",
        source: None,
        default: Some(""),
    },
    Parameter {
        name: "VENDORHOMEPAGELINK",
        source: Some("acapPackageConf.setup.vendorUrl"),
        default: Some(""),
    },
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
        source: Some("acapPackageConf.configuration.httpConfig"),
        default: Some(""),
    },
    Parameter {
        name: "AUTOSTART",
        source: None,
        default: None,
    },
];

fn stringify(value: &Value) -> Vec<(String, Value)> {
    let mut output = Vec::new();
    let mut values = vec![(String::new(), value)];
    while let Some((path, value)) = values.pop() {
        match value {
            Value::Null => output.push((path, Value::Null)),
            Value::Bool(b) => output.push((path, Value::Bool(*b))),
            Value::Number(n) => output.push((path, Value::Number(n.clone()))),
            Value::String(s) => output.push((path, Value::String(s.clone()))),
            Value::Array(a) => {
                for (i, v) in a.iter().enumerate().rev() {
                    let mut p = path.clone();
                    if !p.is_empty() {
                        p.push_str(&format!("[{i}]"));
                    }
                    values.push((p, v));
                }
            }
            Value::Object(o) => {
                for (k, v) in o.iter().rev() {
                    assert!(!k.contains('.'));
                    let mut p = path.clone();
                    if p.is_empty() {
                        p.push_str(&k.to_string());
                    } else {
                        p.push_str(&format!(".{k}"));
                    }
                    values.push((p, v));
                }
            }
        };
    }
    output
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    #[test]
    fn stringify_works_on_example() {
        let value = json!([
            {
                "a": 1,
                "b": [2, 3],
                "c": [{"i":4}],
                "d": {"j": 5},
            },
            "foo",
            Value::Null,
        ]);
        let actual = stringify(&value);
        let expected = vec![
            ("a".to_string(), json!(1)),
            ("b[0]".to_string(), json!(2)),
            ("b[1]".to_string(), json!(3)),
            ("c[0].i".to_string(), json!(4)),
            ("d.j".to_string(), json!(5)),
            ("".to_string(), json!("foo")),
            ("".to_string(), Value::Null),
        ];
        assert_eq!(actual, expected);
    }
}
