use std::{env, path::PathBuf};

use clap::Parser;
use log::debug;

#[derive(Clone, Debug, Parser)]
#[clap(verbatim_doc_comment)]
struct Cli {
    manifest: PathBuf,
    #[clap(short, long)]
    output: Option<PathBuf>,
}

fn main() {
    env_logger::init();
    debug!("Logging initialized");
    let Cli { manifest, output } = Cli::parse();

    dbg!(acap_build::manifest2packageconf(
        &manifest,
        &output.unwrap_or_else(|| env::current_dir().unwrap()),
        &Vec::new(),
    )
    .unwrap());
}
