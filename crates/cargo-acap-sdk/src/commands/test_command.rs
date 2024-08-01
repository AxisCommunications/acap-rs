use cargo_acap_build::{AppBuilder, Architecture, Artifact};
use log::debug;

use crate::{BuildOptions, DeployOptions};

#[derive(clap::Parser, Debug, Clone)]
pub struct TestCommand {
    #[command(flatten)]
    build_options: BuildOptions,
    #[command(flatten)]
    deploy_options: DeployOptions,
}

impl TestCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let Self {
            build_options:
                BuildOptions {
                    target,
                    args: mut build_args,
                },
            deploy_options:
                DeployOptions {
                    host: address,
                    user: username,
                    pass: password,
                },
        } = self;

        build_args.push("--tests".to_string());

        let artifacts = AppBuilder::from_targets([Architecture::from(target)])
            .args(build_args)
            .execute()?;

        for artifact in artifacts {
            debug!("Running {:?}", artifact);
            let envs = vec![("RUST_LOG", "debug"), ("RUST_LOG_STYLE", "always")]
                .into_iter()
                .collect();
            let test_args = ["--test-threads=1"];
            match artifact {
                Artifact::Eap { path, name } => {
                    // TODO: Install instead of patch when needed
                    debug!("Patching app {name}");
                    acap_ssh_utils::patch_package(&path, &username, &password, &address)?;
                    debug!("Running app {name}");
                    acap_ssh_utils::run_package(
                        &username, &password, &address, &name, envs, &test_args,
                    )?
                }
                Artifact::Exe { path } => {
                    debug!(
                        "Running exe {}",
                        path.file_name().unwrap().to_string_lossy()
                    );
                    acap_ssh_utils::run_other(
                        &path, &username, &password, &address, envs, &test_args,
                    )?;
                }
            }
        }
        Ok(())
    }
}
