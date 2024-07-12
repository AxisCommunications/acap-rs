use std::path::PathBuf;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct CargoMetadata {
    pub target_directory: PathBuf,
    pub workspace_root: PathBuf,
}
