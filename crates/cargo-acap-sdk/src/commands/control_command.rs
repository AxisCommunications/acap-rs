use acap_vapix::{applications_control, applications_control::Action};

use crate::DeployOptions;

// TODO: Consider controlling multiple apps with the same selection options used by build etc.
#[derive(clap::Parser, Debug, Clone)]
pub struct ControlCommand {
    /// Name of package to control.
    #[clap(long, short, env = "AXIS_PACKAGE")]
    package: String,
    #[command(flatten)]
    deploy_options: DeployOptions,
}

impl ControlCommand {
    pub async fn exec(self, action: Action) -> anyhow::Result<()> {
        let Self {
            package,
            deploy_options,
        } = self;
        applications_control::control(action, package)
            .execute(&deploy_options.http_client().await?)
            .await?;
        Ok(())
    }
}
