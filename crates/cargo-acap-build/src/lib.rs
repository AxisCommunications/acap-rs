#![doc=include_str!("../README.md")]
use std::path::PathBuf;

pub use acap::Architecture;

mod acap;
mod cargo;
mod cargo_acap;
mod command_utils;

pub use cargo::get_cargo_metadata;
pub fn build(targets: &[Architecture], args: &[&str]) -> anyhow::Result<Vec<PathBuf>> {
    let mut artifacts = Vec::new();
    for target in targets {
        artifacts.extend(cargo_acap::build_and_pack(*target, args)?);
    }
    Ok(artifacts)
}
