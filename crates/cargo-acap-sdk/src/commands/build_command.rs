use cargo_acap_build::{get_cargo_metadata, AppBuilder, Architecture};
use log::debug;

use crate::ResolvedBuildOptions;

#[derive(clap::Parser, Debug, Clone)]
pub struct BuildCommand {
    #[command(flatten)]
    build_options: ResolvedBuildOptions,
}

impl BuildCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let Self {
            build_options:
                ResolvedBuildOptions {
                    target,
                    manifest_path,
                    mut args,
                },
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

        let mut builder = AppBuilder::from_targets([Architecture::from(target)]);
        builder.args(args);
        if let Some(ref path) = manifest_path {
            builder.manifest_path(path);
        }
        builder
            .artifact_dir(
                get_cargo_metadata(manifest_path.as_deref())?
                    .target_directory
                    .join("acap"),
            )
            .execute()?;
        Ok(())
    }
}
