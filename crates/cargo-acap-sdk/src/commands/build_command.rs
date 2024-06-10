use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::{
    acap_utils, cargo_utils, cargo_utils::get_cargo_metadata, docker_utils::DockerOptions,
    BuildOptions,
};

#[derive(clap::Parser, Debug, Clone)]
pub struct BuildCommand {
    #[command(flatten)]
    docker_options: DockerOptions,
    #[command(flatten)]
    build_options: BuildOptions,
}

impl BuildCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let Self {
            mut docker_options,
            build_options,
        } = self;

        docker_options.build_docker_image()?;
        let executables = cargo_utils::build_all(&docker_options, build_options, false, "release")?;

        let apps = acap_utils::stage_and_pack(&docker_options, executables)?;

        // Move the eap files to
        // * make them easier to find,
        // * make it easier to sync the stage dir to the device, and
        // * facilitate transitioning from or to cargo-acap.
        let target_directory = get_cargo_metadata()?.target_directory;
        let final_dir = ensure_final_dir(&target_directory)?;
        for app in apps {
            let to = final_dir.join(app.file_name().context("No file name")?);
            std::fs::copy(app, &to)?;
        }
        Ok(())
    }
}
fn ensure_final_dir(cargo_target_dir: &Path) -> anyhow::Result<PathBuf> {
    let acap_dir = cargo_target_dir.join("acap");
    if !acap_dir.is_dir() {
        std::fs::create_dir(&acap_dir)?;
    }
    Ok(acap_dir)
}
