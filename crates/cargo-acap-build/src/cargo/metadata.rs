// TODO: Consider using `cargo_metadata`
use std::path::PathBuf;

use serde::Deserialize;

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize)]
pub struct CargoMetadata {
    pub packages: Vec<Package>,
    pub target_directory: PathBuf,
    pub workspace_root: PathBuf,
    pub workspace_default_members: Vec<String>,
}

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize)]
pub struct Package {
    pub id: String,
    pub targets: Vec<Target>,
}

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize)]
pub struct Target {
    pub kind: Vec<String>,
    pub name: String,
}
