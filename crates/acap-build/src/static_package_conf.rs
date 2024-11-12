use std::fmt::{Display, Formatter};

use semver::Version;

use crate::{manifest::Manifest, Architecture};

struct PackageConf {
    pub(crate) package_name: String,
    pub(crate) menu_name: Option<String>,
    pub(crate) app_type: String,
    pub(crate) app_name: String,
    pub(crate) app_id: String,
    pub(crate) license_name: String,
    pub(crate) license_page: String,
    pub(crate) license_check_args: Option<String>,
    pub(crate) vendor: String,
    pub(crate) req_emb_dev_version: String,
    pub(crate) app_major_version: String,
    pub(crate) app_minor_version: String,
    pub(crate) app_micro_version: String,
    pub(crate) app_grp: String,
    pub(crate) app_usr: String,
    pub(crate) app_opts: String,
    pub(crate) other_files: String,
    pub(crate) settings_page_file: String,
    pub(crate) settings_page_text: String,
    pub(crate) vendor_homepage_link: String,
    pub(crate) pre_upgrade_script: String,
    pub(crate) post_install_script: String,
    pub(crate) start_mode: String,
    pub(crate) http_cgi_paths: String,
    // autostart cannot exist because
    // - it is not taken from any path in the manifest, and
    // - it has no default value.
}

impl PackageConf {
    pub(crate) fn _from_manifest(manifest: &Manifest, arch: Architecture) -> anyhow::Result<Self> {
        let app_version = Version::parse(&manifest.acap_package_conf.setup.version)?;
        let app_name = manifest.acap_package_conf.setup.app_name.to_string();
        let package_name = manifest
            .acap_package_conf
            .setup
            .friendly_name
            .as_deref()
            .unwrap_or(app_name.as_str())
            .replace(' ', "_");

        Ok(Self {
            package_name,
            menu_name: None,
            app_type: (|| {
                Some(
                    manifest
                        .acap_package_conf
                        .setup
                        .architecture
                        .as_ref()?
                        .as_str(),
                )
            })()
            .unwrap_or(arch.nickname())
            .to_string(),
            app_name,
            app_id: (|| Some(manifest.acap_package_conf.setup.app_id.as_ref()?.as_str()))()
                .unwrap_or("")
                .to_string(),
            license_name: "".to_string(),
            license_page: (|| {
                Some(
                    manifest
                        .acap_package_conf
                        .copy_protection
                        .as_ref()?
                        .method
                        .as_str(),
                )
            })()
            .unwrap_or("none")
            .to_string(),
            license_check_args: None,
            vendor: "".to_string(),
            req_emb_dev_version: "".to_string(),
            app_major_version: app_version.major.to_string(),
            app_minor_version: app_version.minor.to_string(),
            app_micro_version: app_version.patch.to_string(),
            app_grp: (|| {
                Some(
                    manifest
                        .acap_package_conf
                        .setup
                        .user
                        .as_ref()?
                        .group
                        .as_str(),
                )
            })()
            .unwrap_or("sdk")
            .to_string(),
            app_usr: (|| {
                Some(
                    manifest
                        .acap_package_conf
                        .setup
                        .user
                        .as_ref()?
                        .username
                        .as_str(),
                )
            })()
            .unwrap_or("sdk")
            .to_string(),
            app_opts: (|| {
                Some(
                    manifest
                        .acap_package_conf
                        .setup
                        .run_options
                        .as_ref()?
                        .as_str(),
                )
            })()
            .unwrap_or("")
            .to_string(),
            other_files: "".to_string(),
            settings_page_file: "".to_string(),
            settings_page_text: "".to_string(),
            vendor_homepage_link: "".to_string(),
            pre_upgrade_script: "".to_string(),
            post_install_script: "".to_string(),
            start_mode: "".to_string(),
            http_cgi_paths: if (|| {
                manifest
                    .acap_package_conf
                    .configuration
                    .as_ref()?
                    .http_config
                    .as_ref()
            })()
            .map(Vec::is_empty)
            .unwrap_or(true)
            {
                ""
            } else {
                "cgi.conf"
            }
            .to_string(),
        })
    }
}

impl Default for PackageConf {
    fn default() -> Self {
        Self {
            package_name: "".to_string(),
            menu_name: None,
            app_type: "".to_string(),
            app_name: "".to_string(),
            app_id: "".to_string(),
            license_name: "Available".to_string(),
            license_page: "none".to_string(),
            license_check_args: None,
            vendor: "-".to_string(),
            req_emb_dev_version: "2.0".to_string(),
            app_major_version: "1".to_string(),
            app_minor_version: "0".to_string(),
            app_micro_version: "0".to_string(),
            app_grp: "sdk".to_string(),
            app_usr: "sdk".to_string(),
            app_opts: "".to_string(),
            other_files: "".to_string(),
            settings_page_file: "".to_string(),
            settings_page_text: "".to_string(),
            vendor_homepage_link: "".to_string(),
            pre_upgrade_script: "".to_string(),
            post_install_script: "".to_string(),
            start_mode: "never".to_string(),
            http_cgi_paths: "".to_string(),
        }
    }
}

impl Display for PackageConf {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "=\"{}\"", self.package_name)?;
        if let Some(menu_name) = &self.menu_name {
            write!(f, "=\"{}\"", menu_name)?;
        }
        write!(f, "=\"{}\"", self.app_type)?;
        write!(f, "=\"{}\"", self.app_name)?;
        write!(f, "=\"{}\"", self.app_id)?;
        write!(f, "=\"{}\"", self.license_name)?;
        write!(f, "=\"{}\"", self.license_page)?;
        if let Some(license_check_args) = &self.license_check_args {
            write!(f, "=\"{}\"", license_check_args)?;
        }
        write!(f, "=\"{}\"", self.vendor)?;
        write!(f, "=\"{}\"", self.req_emb_dev_version)?;
        write!(f, "=\"{}\"", self.app_major_version)?;
        write!(f, "=\"{}\"", self.app_minor_version)?;
        write!(f, "=\"{}\"", self.app_micro_version)?;
        write!(f, "=\"{}\"", self.app_grp)?;
        write!(f, "=\"{}\"", self.app_usr)?;
        write!(f, "=\"{}\"", self.app_opts)?;
        write!(f, "=\"{}\"", self.other_files)?;
        write!(f, "=\"{}\"", self.settings_page_file)?;
        write!(f, "=\"{}\"", self.settings_page_text)?;
        write!(f, "=\"{}\"", self.vendor_homepage_link)?;
        write!(f, "=\"{}\"", self.pre_upgrade_script)?;
        write!(f, "=\"{}\"", self.post_install_script)?;
        write!(f, "=\"{}\"", self.start_mode)?;
        write!(f, "=\"{}\"", self.http_cgi_paths)?;
        Ok(())
    }
}
