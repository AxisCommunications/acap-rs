use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use log::info;
use tempfile::NamedTempFile;
const CONFIG: &str = include_str!("../../../../about.toml");
const TEMPLATE: &str = include_str!("../../../../about.hbs");

#[derive(Debug, Hash)]
struct Key<'a> {
    config: &'a str,
    template: &'a str,
    manifest: &'a str,
    version: &'a str,
}

pub(crate) fn generate(manifest_file: &Path, cache: &Path) -> anyhow::Result<PathBuf> {
    let mut config_file = NamedTempFile::new()?;
    config_file.write_all(CONFIG.as_bytes())?;

    let mut template_file = NamedTempFile::new()?;
    template_file.write_all(TEMPLATE.as_bytes())?;

    let manifest = fs::read_to_string(manifest_file)?;

    // TODO: Consider installing cargo about locally.
    let version = String::from_utf8(
        Command::new("cargo-about")
            .arg("--version")
            .output()?
            .stdout,
    )?;

    let mut hasher = DefaultHasher::new();
    Key {
        config: CONFIG,
        template: TEMPLATE,
        // TODO: Use lock file as well or instead.
        manifest: manifest.as_str(),
        version: version.as_str(),
    }
    .hash(&mut hasher);

    let cache_name = format!("{:016x}", hasher.finish());
    let cache_file = cache.join(&cache_name);
    if !cache_file.exists() {
        // TODO: Reimplement using the library crate or using artifact dependencies when ready.
        info!("Cache {cache_name} not found, generating new cache for {manifest_file:?}");
        let status = Command::new("cargo-about")
            .arg("generate")
            .arg("--fail")
            .arg("--manifest-path")
            .arg(manifest_file)
            .arg("--output-file")
            .arg(cache_file.as_os_str())
            .arg("--config")
            .arg(config_file.path().as_os_str())
            .arg(template_file.path().as_os_str())
            .spawn()?
            .wait()?;
        assert!(status.success());
    }
    assert!(cache_file.exists());
    Ok(cache_file)
}
