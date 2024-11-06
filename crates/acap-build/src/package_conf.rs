use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
};

use anyhow::bail;
use log::debug;
use semver::Version;
use serde_json::Value;

use crate::Architecture;

#[derive(Clone, Debug)]
pub(crate) struct PackageConf {
    pub(crate) parameters: Vec<(String, String)>,
    default_arch: Architecture,
}
impl PackageConf {
    fn push(&mut self, key: impl ToString, value: impl ToString) {
        self.parameters.push((key.to_string(), value.to_string()));
    }

    pub fn new_from_manifest(
        manifest: &Value,
        outpath: &Path,
        otherfiles: &[PathBuf],
        arch: Architecture,
    ) -> anyhow::Result<PackageConf> {
        let aliases: HashMap<_, _> = [
            ("acapPackageConf.setup.user.group", "APPGRP"),
            ("acapPackageConf.setup.appId", "APPID"),
            ("acapPackageConf.setup.version", "APPMAJORVERSION"),
            ("acapPackageConf.setup.appName", "APPNAME"),
            ("acapPackageConf.setup.runOptions", "APPOPTS"),
            ("acapPackageConf.setup.architecture", "APPTYPE"),
            ("acapPackageConf.setup.user.username", "APPUSR"),
            ("acapPackageConf.configuration.httpConfig", "HTTPCGIPATHS"),
            ("acapPackageConf.copyProtection.method", "LICENSEPAGE"),
            (
                "acapPackageConf.copyProtection.customOptions",
                "LICENSE_CHECK_ARGS",
            ),
            ("acapPackageConf.setup.friendlyName", "PACKAGENAME"),
            (
                "acapPackageConf.installation.postInstallScript",
                "POSTINSTALLSCRIPT",
            ),
            (
                "acapPackageConf.setup.embeddedSdkVersion",
                "REQEMBDEVVERSION",
            ),
            (
                "acapPackageConf.configuration.settingPage",
                "SETTINGSPAGEFILE",
            ),
            ("acapPackageConf.setup.runMode", "STARTMODE"),
            ("acapPackageConf.setup.vendor", "VENDOR"),
            ("acapPackageConf.setup.vendorUrl", "VENDORHOMEPAGELINK"),
        ]
        .into_iter()
        .collect();

        let parameters = stringify(manifest);
        let mut entries = Self {
            parameters: Vec::new(),
            default_arch: arch,
        };
        let mut cgi_parsed = false;
        for (key, value) in parameters {
            let Value::String(value) = value else {
                bail!("Expected {key} version to be a string")
            };
            match key.as_str() {
                "acapPackageConf.setup.version" => {
                    let v = Version::parse(&value)?;
                    entries.push("APPMAJORVERSION", v.major);
                    entries.push("APPMINORVERSION", v.minor);
                    entries.push("APPMICROVERSION", v.patch);
                }
                "acapPackageConf.setup.vendorUrl" => {
                    let re = regex::Regex::new("(?:(?:http|https)://)?(.+)")
                        .expect("Hard-coded regex is valid");
                    let Some(caps) = re.captures(&value) else {
                        bail!("Expected vendor url to match regex {:?}", re)
                    };
                    let domain_name = caps
                        .get(1)
                        .expect("Hard coded regex as exactly one capture group")
                        .as_str();
                    entries.push(
                        "VENDORHOMEPAGELINK",
                        format!(r#"<a href="{value}" target="_blank">{domain_name}</a>"#),
                    );
                }
                k if k.starts_with("acapPackageConf.configuration.httpConfig") => {
                    if !cgi_parsed {
                        cgi_parsed = true;
                        entries.push("HTTPCGIPATHS", "cgi.conf");
                    }
                }
                k => {
                    if let Some(pkg_key) = aliases.get(k) {
                        entries.push(pkg_key, value);
                    } else {
                        debug!("{k} skipped, no corresponding parameter in package.conf")
                    }
                }
            }
        }

        if !otherfiles.is_empty() {
            let mut relpaths = Vec::new();
            for file in otherfiles {
                let relpath = match file.is_absolute() {
                    true => file.strip_prefix(outpath)?,
                    false => file,
                };
                relpaths.push(relpath.to_string_lossy().to_string());
            }
            entries.push("OTHERFILES", relpaths.join(" "));
        }

        Ok(entries)
    }

    pub fn file_name() -> &'static str {
        "package.conf"
    }
}

// PACKAGENAME
// MENUNAME
// APPTYPE
// APPNAME
// APPID
// LICENSENAME
// LICENSEPAGE
// LICENSE_CHECK_ARGS
// VENDOR
// REQEMBDEVVERSION
// APPMAJORVERSION
// APPMINORVERSION
// APPMICROVERSION
// APPGRP
// APPUSR
// APPOPTS
// OTHERFILES
// SETTINGSPAGEFILE
// SETTINGSPAGETEXT
// VENDORHOMEPAGELINK
// PREUPGRADESCRIPT
// POSTINSTALLSCRIPT
// STARTMODE
// HTTPCGIPATHS

// Fusion of eap-create.sh and package-conf-parameters.cfg
const PARAMETERS: [(&str, Option<&str>); 25] = [
    ("PACKAGENAME", Some("")),
    ("MENUNAME", None),
    ("APPTYPE", Some("")),
    ("APPNAME", Some("")),
    ("APPID", Some("")),
    ("LICENSENAME", Some("Available")),
    ("LICENSEPAGE", Some("none")),
    ("LICENSE_CHECK_ARGS", None),
    ("VENDOR", Some("-")),
    ("REQEMBDEVVERSION", Some("2.0")),
    ("APPMAJORVERSION", Some("1")),
    ("APPMINORVERSION", Some("0")),
    ("APPMICROVERSION", Some("0")),
    ("APPGRP", Some("sdk")),
    ("APPUSR", Some("sdk")),
    ("APPOPTS", Some("")),
    ("OTHERFILES", Some("")),
    ("SETTINGSPAGEFILE", Some("")),
    ("SETTINGSPAGETEXT", Some("")),
    ("VENDORHOMEPAGELINK", Some("")),
    ("PREUPGRADESCRIPT", Some("")),
    ("POSTINSTALLSCRIPT", Some("")),
    ("STARTMODE", Some("never")),
    ("HTTPCGIPATHS", Some("")),
    ("AUTOSTART", None),
];

impl Display for PackageConf {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        dbg!(self);
        let default_arch = self.default_arch.nickname();
        let lut: HashMap<_, _> = self
            .parameters
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        for (key, default) in PARAMETERS {
            let value = match lut.get(key) {
                Some(v) => Some(v),
                None => match key {
                    "PACKAGENAME" => lut.get("APPNAME"),
                    "APPTYPE" => Some(&default_arch),
                    _ => default.as_ref(),
                },
            };
            if let Some(value) = value {
                if key == "VENDORHOMEPAGELINK" {
                    writeln!(f, r#"{key}='{value}'"#)?;
                } else {
                    writeln!(f, r#"{key}="{value}""#)?;
                }
            }
        }
        Ok(())
    }
}

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
