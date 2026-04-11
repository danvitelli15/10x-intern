use anyhow::Result;
use clap::Parser;

use intern::cli::Cli;
use intern::config::Config;
use intern::orchestrator;

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let config = Config::load()?;

    orchestrator::run(cli.command, config)
}
