use clap::{Command, Parser};
use clap_complete::{generate, Shell};

#[derive(Debug, Parser)]
pub struct CompletionsCommand {
    shell: Shell,
}

impl CompletionsCommand {
    pub fn exec(self, mut cmd: Command) -> anyhow::Result<()> {
        let Self { shell } = self;
        let name = cmd.get_name().to_string();
        generate(shell, &mut cmd, name, &mut std::io::stdout());
        Ok(())
    }
}
