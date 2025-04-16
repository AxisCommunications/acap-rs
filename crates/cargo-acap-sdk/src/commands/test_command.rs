use cargo_acap_build::{AppBuilder, Architecture, Artifact};
use log::debug;
use ssh2::Session;
use std::net::TcpStream;

use crate::{BuildOptions, DeployOptions, ResolvedBuildOptions};

#[derive(clap::Parser, Debug, Clone)]
pub struct TestCommand {
    #[command(flatten)]
    build_options: BuildOptions,
    #[command(flatten)]
    deploy_options: DeployOptions,
}

impl TestCommand {
    pub async fn exec(self) -> anyhow::Result<()> {
        let Self {
            build_options,
            deploy_options,
        } = self;

        let ResolvedBuildOptions {
            target,
            args: mut build_args,
        } = build_options.resolve(&deploy_options).await?;

        let DeployOptions {
            host: address,
            user: username,
            pass: password,
        } = deploy_options;

        build_args.push("--tests".to_string());

        let tcp = TcpStream::connect(format!("{}:22", address)).unwrap();
        let mut session = Session::new().unwrap();
        session.set_tcp_stream(tcp);
        session.handshake().unwrap();

        session.userauth_password(&username, &password).unwrap();

        let artifacts = AppBuilder::from_targets([Architecture::from(target)])
            .args(build_args)
            .execute()?;

        for artifact in artifacts {
            debug!("Running {:?}", artifact);
            let envs = [("RUST_LOG", "debug"), ("RUST_LOG_STYLE", "always")];
            let test_args = ["--test-threads=1"];
            match artifact {
                Artifact::Eap { path, name } => {
                    // TODO: Install instead of patch when needed
                    debug!("Patching app {name}");
                    acap_ssh_utils::patch_package(&path, &session)?;
                    debug!("Running app {name}");
                    acap_ssh_utils::run_package(
                        &session,
                        &name,
                        &envs,
                        &test_args,
                        username != "root",
                    )?
                }
                Artifact::Exe { path } => {
                    debug!(
                        "Running exe {}",
                        path.file_name().unwrap().to_string_lossy()
                    );
                    acap_ssh_utils::run_other(&path, &session, &envs, &test_args)?;
                }
            }
        }
        Ok(())
    }
}
