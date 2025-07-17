use cargo_acap_build::{AppBuilder, Architecture, Artifact};
use log::debug;

use crate::{BuildOptions, DeployOptions, ResolvedBuildOptions};

#[derive(clap::Parser, Debug, Clone)]
pub struct RunCommand {
    #[command(flatten)]
    build_options: BuildOptions,
    #[command(flatten)]
    deploy_options: DeployOptions,
}

impl RunCommand {
    pub async fn exec(self) -> anyhow::Result<()> {
        let Self {
            build_options,
            deploy_options,
        } = self;

        let ResolvedBuildOptions { target, args } = build_options.resolve(&deploy_options).await?;

        let DeployOptions {
            host: address,
            http_port: _,
            https_port: _,
            ssh_port,
            user: username,
            ssh_user: username,
            pass: password,
        } = deploy_options;

        let artifacts = AppBuilder::from_targets([Architecture::from(target)])
            .args(args)
            .execute()?;
        for artifact in artifacts {
            let envs = vec![("RUST_LOG", "debug"), ("RUST_LOG_STYLE", "always")]
                .into_iter()
                .collect();
            match artifact {
                Artifact::Eap { path, name } => {
                    let username = DeployOptions::username_for_eap(&username, &name);
                    // TODO: Install instead of patch when needed
                    debug!("Patching app {name}");
                    acap_ssh_utils::patch_package(&path, &username, &password, &address, ssh_port)?;
                    debug!("Running app {name}");
                    acap_ssh_utils::run_package(
                        &username,
                        &password,
                        &address,
                        ssh_port,
                        &name,
                        envs,
                        &[],
                    )?
                }
                Artifact::Exe { path } => {
                    let username = DeployOptions::username_for_exe();
                    debug!(
                        "Running exe {}",
                        path.file_name().unwrap().to_string_lossy()
                    );
                    acap_ssh_utils::run_other(
                        &path,
                        &username,
                        &password,
                        &address,
                        ssh_port,
                        envs,
                        &[],
                    )?;
                }
            }
        }
        Ok(())
    }
}
