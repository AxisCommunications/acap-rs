use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "reason")]
pub enum JsonMessage {
    #[serde(rename = "compiler-artifact")]
    CompilerArtifact {
        package_id: String,
        manifest_path: PathBuf,
        executable: Option<PathBuf>,
        target: Target,
    },
    // We don't care about the content of these for now, but include them so that the parsing
    // succeeds.
    #[serde(rename = "compiler-message")]
    CompilerMessage { message: String },
    #[serde(rename = "build-script-executed")]
    BuildScriptExecuted {
        package_id: String,
        out_dir: PathBuf,
    },
    #[serde(rename = "build-finished")]
    BuildFinished { success: bool },
}

#[derive(Clone, Debug, Deserialize)]
pub struct Target {
    pub name: String,
}
