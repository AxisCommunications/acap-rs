use std::path::Path;

use anyhow::{bail, Context};

use crate::{
    acap_utils, cargo_utils, command_utils::RunWith, docker_utils::DockerOptions, ArchAbi,
    BuildOptions, DeployOptions,
};

#[derive(clap::Parser, Debug, Clone)]
pub struct InstallCommand {
    #[command(flatten)]
    docker_options: DockerOptions,
    #[command(flatten)]
    build_options: BuildOptions,
    #[command(flatten)]
    deploy_options: DeployOptions,
}

impl InstallCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let Self {
            mut docker_options,
            build_options,
            deploy_options,
        } = self;

        let mut targets = build_options.target.iter();
        let &target = targets
            .next()
            .context("Expected exactly one target but got zero")?;
        if let Some(extra) = targets.next() {
            bail!("Expected exactly one target but got at least two ({target}, {extra})")
        }
        docker_options.build_docker_image()?;
        let executables = cargo_utils::build_all(&docker_options, build_options, false, "release")?;
        let apps = acap_utils::stage_and_pack(&docker_options, executables)?;
        for app in apps {
            install_one(&docker_options, target, &app, &deploy_options)?;
        }
        Ok(())
    }
}

pub fn install_one(
    docker_options: &DockerOptions,
    architecture: ArchAbi,
    app: &Path,
    deploy_options: &DeployOptions,
) -> anyhow::Result<()> {
    let mut eap_install = std::process::Command::new("eap-install.sh");
    eap_install.arg(deploy_options.address.to_string());
    eap_install.arg(&deploy_options.password);
    eap_install.arg("install");
    assert_eq!(&deploy_options.username, "root");

    let app_dir = app.parent().context("app not in a directory")?;
    let mut sh = docker_options.command(app_dir, "sh", false)?;

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
