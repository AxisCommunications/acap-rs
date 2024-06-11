use std::io::{stderr, stdin, stdout, IsTerminal};

use clap::Parser;
use log::debug;

use crate::docker_utils::DockerOptions;

#[derive(Debug, Parser)]
pub struct ContainerizeCommand {
    #[command(flatten)]
    docker_options: DockerOptions,
    args: Vec<String>,
}

impl ContainerizeCommand {
    pub fn exec(self) -> anyhow::Result<()> {
        let Self {
            mut docker_options,
            mut args,
        } = self;
        docker_options.build_docker_image()?;
        let current_dir = std::env::current_dir()?;
        let interactive_tty =
            stdin().is_terminal() && stdout().is_terminal() && stderr().is_terminal();
        let program = match args.len() {
            0 => None,
            _ => Some(args.remove(0)),
        };
        let mut program =
            docker_options.docker_command(&current_dir, program.as_deref(), interactive_tty)?;
        program.args(&args);
        debug!("Running command {program:#?}...");
        if let Some(code) = program.status()?.code() {
            std::process::exit(code);
        }
        Ok(())
    }
}
