use acap_vapix::{applications_control, applications_control::Action};
use anyhow::{bail, Context};
use cargo_acap_build::get_cargo_metadata;

use crate::DeployOptions;

// TODO: Consider controlling multiple apps with the same selection options used by build etc.
#[derive(clap::Parser, Debug, Clone)]
pub struct ControlCommand {
    // No other commands currently allow package selection to be controlled with `AXIS_PACKAGE`.
    // As long as the `Makefile` is important and uses the variable, it makes some sense to have it,
    // but after that I don't know.
    // TODO: Implement consistent package selection across commands.
    /// Name of package to control.
    #[clap(long, short, env = "AXIS_PACKAGE")]
    package: Option<String>,
    #[command(flatten)]
    deploy_options: DeployOptions,
}

impl ControlCommand {
    pub async fn exec(self, action: Action) -> anyhow::Result<()> {
        let Self {
            package,
            deploy_options,
        } = self;
        let package = match package {
            None => infer_name_from_metadata().context("could not infer package name")?,
            Some(p) => p,
        };
        // TODO: Improve error messages
        applications_control::control(action, package)
            .execute(&deploy_options.http_client().await?)
            .await?;
        Ok(())
    }
}

/// Return the name of the binary target if there is only one.
///
/// This is similar to how cargo selects targets when `--package` is not specified.
///
/// One known deviation is that `cargo uninstall`, which also operates only on binary crates, will
/// not work from the workspace root if it is not also the root of the binary crate whereas this
/// will.
fn infer_name_from_metadata() -> anyhow::Result<String> {
    let metadata = get_cargo_metadata()?;

    let find_binary_targets = |id: &String| {
        metadata
            .packages
            .iter()
            .find(|p| p.id == *id)
            .expect("Cargo output is trusted and all workspace members are packages")
            .targets
            .iter()
            .filter(|t| t.kind.iter().any(|k| k == "bin"))
    };

    let candidates = metadata
        .workspace_default_members
        .iter()
        .flat_map(find_binary_targets)
        .collect::<Vec<_>>();

    match candidates.len() {
        0 => bail!("zero workspace members"),
        1 => Ok(candidates[0]
            .name
            .rsplit('/')
            .next()
            .expect("Split generates at least one substring")
            .split('#')
            .next()
            .expect("Split generates at least one substring")
            .to_string()),
        _ => bail!("more than one workspace member"),
    }
}
