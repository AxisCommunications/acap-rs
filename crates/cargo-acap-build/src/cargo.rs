/// Wrapper around `cargo`.
use metadata::CargoMetadata;

use crate::command_utils::RunWith;

pub mod json_message;
pub mod metadata;

pub fn get_cargo_metadata() -> anyhow::Result<CargoMetadata> {
    let mut cargo = std::process::Command::new("cargo");
    cargo.arg("metadata");
    cargo.args(["--format-version", "1"]);
    let metadata = cargo.run_with_captured_stdout()?;
    let metadata: CargoMetadata = serde_json::from_str(&metadata)?;
    Ok(metadata)
}
