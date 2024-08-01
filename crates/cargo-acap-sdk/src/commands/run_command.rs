use cargo_acap_build::{AppBuilder, Architecture, Artifact};
use log::debug;

use crate::{BuildOptions, DeployOptions};

#[derive(clap::Parser, Debug, Clone)]
pub struct RunCommand {
    #[command(flatten)]
    build_options: BuildOptions,
    #[command(flatten)]
    deploy_options: DeployOptions,
}

impl RunCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let Self {
            build_options: BuildOptions { target, args },
            deploy_options:
                DeployOptions {
                    host: address,
                    user: username,
                    pass: password,
                },
        } = self;

        let artifacts = AppBuilder::from_targets([Architecture::from(target)])
            .args(args)
            .execute()?;
        for artifact in artifacts {
            let envs = vec![("RUST_LOG", "debug"), ("RUST_LOG_STYLE", "always")]
                .into_iter()
                .collect();
            match artifact {
                Artifact::Eap { path, name } => {
                    // TODO: Install instead of patch when needed
                    debug!("Patching app {name}");
                    acap_ssh_utils::patch_package(&path, &username, &password, &address)?;
                    debug!("Running app {name}");
                    acap_ssh_utils::run_package(&username, &password, &address, &name, envs, &[])?
                }
                Artifact::Exe { path } => {
                    debug!(
                        "Running exe {}",
                        path.file_name().unwrap().to_string_lossy()
                    );
                    acap_ssh_utils::run_other(&path, &username, &password, &address, envs, &[])?;
                }
            }
        }
        Ok(())
    }
}
