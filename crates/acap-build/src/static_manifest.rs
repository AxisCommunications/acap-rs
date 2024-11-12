use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcapPackageConf {
    pub setup: Setup,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) configuration: Option<Configuration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) copy_protection: Option<CopyProtection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) uninstallation: Option<Uninstallation>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Configuration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) http_config: Option<Vec<HttpConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reverse_proxy: Option<Vec<ReverseProxy>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) setting_page: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) param_config: Option<Vec<ParamConfig>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CopyProtection {
    pub(crate) method: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DbusResources {
    pub(crate) required_methods: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HttpConfig {
    pub(crate) access: String,
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LinuxResources {
    pub(crate) user: LinuxUser,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LinuxUser {
    pub(crate) groups: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub(crate) schema_version: String,
    pub acap_package_conf: AcapPackageConf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) resources: Option<Resources>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParamConfig {
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) default: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Resources {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) dbus: Option<DbusResources>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) linux: Option<LinuxResources>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReverseProxy {
    pub(crate) api_path: String,
    pub(crate) target: String,
    pub(crate) access: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Setup {
    pub app_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) friendly_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) app_id: Option<String>,
    pub(crate) vendor: String,
    pub(crate) run_mode: String,
    pub(crate) version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) architecture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) user: Option<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_options: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Uninstallation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pre_uninstall_script: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct User {
    pub(crate) username: String,
    pub(crate) group: String,
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
