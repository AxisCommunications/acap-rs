use acap_vapix::applications_upload;
use cargo_acap_build::{AppBuilder, Architecture, Artifact};
use log::debug;

use crate::{BuildOptions, DeployOptions, ResolvedBuildOptions};

#[derive(clap::Parser, Debug, Clone)]
pub struct InstallCommand {
    #[command(flatten)]
    build_options: BuildOptions,
    #[command(flatten)]
    deploy_options: DeployOptions,
}

impl InstallCommand {
    pub async fn exec(self) -> anyhow::Result<()> {
        let Self {
            build_options,
            deploy_options,
        } = self;

        let ResolvedBuildOptions { target, mut args } =
            build_options.resolve(&deploy_options).await?;

        if !args.iter().any(|arg| {
            arg.split('=')
                .next()
                .expect("Split always yields at least one substring")
                .starts_with("--profile")
        }) {
            debug!("Using release profile by default");
            args.push("--profile=release".to_string());
        }

        let artifacts = AppBuilder::from_targets([Architecture::from(target)])
            .args(args)
            .execute()?;

        // TODO: Handle the case where multiple artifacts of the same kind have the same name.
        for artifact in artifacts {
            match artifact {
                Artifact::Eap { path, name } => {
                    debug!("Installing app {name} from {path:?}");
                    applications_upload::Client::new(&deploy_options.http_client().await?)
                        .upload(path)?
                        .send()
                        .await?;
                    debug!("Installed app {name}");
                }
                Artifact::Exe { path } => {
                    debug!("Skipping exe {path:?}");
                }
            }
        }
        Ok(())
    }
}
