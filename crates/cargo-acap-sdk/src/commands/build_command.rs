use cargo_acap_build::{get_cargo_metadata, AppBuilder, Architecture};
use log::debug;

use crate::BuildOptions;

#[derive(clap::Parser, Debug, Clone)]
pub struct BuildCommand {
    #[command(flatten)]
    build_options: BuildOptions,
}

impl BuildCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let Self {
            build_options: BuildOptions { target, mut args },
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

        AppBuilder::from_targets([Architecture::from(target)])
            .args(args)
            .artifact_dir(get_cargo_metadata()?.target_directory.join("acap"))
            .execute()?;
        Ok(())
    }
}
