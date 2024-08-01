use std::path::Path;

use anyhow::Context;
use cargo_acap_build::{AppBuilder, Architecture, Artifact};
use log::debug;

use crate::{command_utils::RunWith, ArchAbi, BuildOptions, DeployOptions};

#[derive(clap::Parser, Debug, Clone)]
pub struct InstallCommand {
    #[command(flatten)]
    build_options: BuildOptions,
    #[command(flatten)]
    deploy_options: DeployOptions,
}

impl InstallCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let Self {
            build_options: BuildOptions { target, mut args },
            deploy_options,
        } = self;

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
                Artifact::Eap { path, .. } => install_one(target, &path, &deploy_options).unwrap(),
                Artifact::Exe { path } => {
                    debug!("Skipping exe {path:?}")
                }
            }
        }
        Ok(())
    }
}

pub fn install_one(
    architecture: ArchAbi,
    app: &Path,
    deploy_options: &DeployOptions,
) -> anyhow::Result<()> {
    // TODO: Run in temporary directory or reimplement in rust for better control.
    let mut eap_install = std::process::Command::new("eap-install.sh");
    eap_install.arg(deploy_options.host.to_string());
    eap_install.arg(&deploy_options.pass);
    eap_install.arg("install");
    assert_eq!(&deploy_options.user, "root");

    let app_dir = app.parent().context("app not in a directory")?;
    let mut sh = std::process::Command::new("sh");
    sh.current_dir(app_dir);

    let env_setup = match architecture {
        ArchAbi::Aarch64 => "environment-setup-cortexa53-crypto-poky-linux",
        ArchAbi::Armv7hf => "environment-setup-cortexa9hf-neon-poky-linux-gnueabi",
    };
    sh.args([
        "-c",
        &format!(". /opt/axis/acapsdk/{env_setup} && {:?}", eap_install),
    ]);
    sh.run_with_logged_stdout()?;
    Ok(())
}
