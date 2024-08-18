use acap_vapix::{application, application::Action};

use crate::DeployOptions;

pub enum Start {}
pub enum Stop {}
pub enum Restart {}
pub enum Remove {}

pub trait IntoAction {
    fn action() -> Action;
}

impl IntoAction for Start {
    fn action() -> Action {
        Action::Start
    }
}
impl IntoAction for Stop {
    fn action() -> Action {
        Action::Stop
    }
}
impl IntoAction for Restart {
    fn action() -> Action {
        Action::Restart
    }
}
impl IntoAction for Remove {
    fn action() -> Action {
        Action::Remove
    }
}

// TODO: Enable starting multiple apps
#[derive(clap::Parser, Debug, Clone)]
pub struct ControlCommand {
    /// Name of package to control.
    #[clap(long, short, env = "AXIS_PACKAGE")]
    package: String,
    #[command(flatten)]
    deploy_options: DeployOptions,
}

impl ControlCommand {
    pub async fn exec<T: IntoAction>(self) -> anyhow::Result<()> {
        let Self {
            package,
            deploy_options,
        } = self;
        application::control(T::action(), package)
            .execute(&deploy_options.http_client().await?)
            .await?;
        Ok(())
    }
}
