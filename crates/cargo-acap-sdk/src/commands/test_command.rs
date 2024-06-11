use anyhow::{bail, Context};
use log::debug;

use crate::{
    acap_utils, acap_utils::StagedApp, cargo_utils, docker_utils::DockerOptions, BuildOptions,
    DeployOptions,
};

#[derive(clap::Parser, Debug, Clone)]
pub struct TestCommand {
    #[command(flatten)]
    docker_options: DockerOptions,
    #[command(flatten)]
    build_options: BuildOptions,
    #[command(flatten)]
    deploy_options: DeployOptions,
}

impl TestCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let Self {
            mut docker_options,
            build_options,
            deploy_options:
                DeployOptions {
                    address,
                    username,
                    password,
                },
        } = self;

        let mut targets = build_options.target.iter();
        let &target = targets
            .next()
            .context("Expected exactly one target but got zero")?;
        if let Some(extra) = targets.next() {
            bail!("Expected exactly one target but got at least two ({target}, {extra})")
        }

        docker_options.build_docker_image()?;
        let artifacts = cargo_utils::build_all(&docker_options, build_options, true, "dev")?;
        for artifact in artifacts {
            debug!("Running test {:?}", artifact.executable);
            let envs = vec![("RUST_LOG", "debug"), ("RUST_LOG_STYLE", "always")]
                .into_iter()
                .collect();
            let args = ["--test-threads", "1"];
            if acap_utils::app_dir(&artifact.manifest_path).is_some() {
                let staged_app = StagedApp::try_new(artifact)?;
                let app_name = staged_app.sync(&username, &password, &address)?;
                acap_ssh_utils::run_as_package(
                    &username, &password, &address, &app_name, envs, &args,
                )?;
            } else {
                acap_ssh_utils::run_as_self(
                    &artifact.executable,
                    &username,
                    &password,
                    &address,
                    envs,
                    &args,
                )?;
            }
        }
        Ok(())
    }
}
