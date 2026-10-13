/// Wrapper around `cargo`.
use std::path::Path;

use metadata::CargoMetadata;

use crate::command_utils::RunWith;

pub mod json_message;
pub mod metadata;

pub(crate) fn cargo_command(
    subcommand: &str,
    manifest_path: Option<&Path>,
) -> std::process::Command {
    let mut cmd = std::process::Command::new("cargo");
    // `--manifest-path` is an option of the subcommand, so it must come after it.
    cmd.arg(subcommand);
    if let Some(path) = manifest_path {
        cmd.arg("--manifest-path");
        cmd.arg(path);
    }
    cmd
}

pub fn get_cargo_metadata(manifest_path: Option<&Path>) -> anyhow::Result<CargoMetadata> {
    let mut cargo = cargo_command("metadata", manifest_path);
    cargo.args(["--format-version", "1"]);
    let metadata = cargo.run_with_captured_stdout()?;
    let metadata: CargoMetadata = serde_json::from_str(&metadata)?;
    Ok(metadata)
}
