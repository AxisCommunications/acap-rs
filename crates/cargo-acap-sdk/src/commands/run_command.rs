use cargo_acap_build::{AppBuilder, Architecture, Artifact};
use log::debug;
use ssh2::Session;
use std::net::TcpStream;

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
            user: username,
            pass: password,
        } = deploy_options;

        let host = format!("{}:22", address);

        let tcp = TcpStream::connect(host)?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake().unwrap();

        session.userauth_password(&username, &password)?;

        let artifacts = AppBuilder::from_targets([Architecture::from(target)])
            .args(args)
            .execute()?;
        for artifact in artifacts {
            let envs = [("RUST_LOG", "debug"), ("RUST_LOG_STYLE", "always")];
            match artifact {
                Artifact::Eap { path, name } => {
                    // TODO: Install instead of patch when needed
                    debug!("Patching app {name}");
                    acap_ssh_utils::patch_package(&path, &session)?;
                    debug!("Running app {name}");
                    acap_ssh_utils::run_package(&session, &name, &envs, &[], username != "root")?
                }
                Artifact::Exe { path } => {
                    debug!(
                        "Running exe {}",
                        path.file_name().unwrap().to_string_lossy()
                    );
                    acap_ssh_utils::run_other(&path, &session, &envs, &[])?;
                }
            }
        }
        Ok(())
    }
}
