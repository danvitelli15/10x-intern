use anyhow::Result;
use clap::Parser;

use intern::cli::Cli;
use intern::config::Config;
use intern::orchestrator;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| {
            use std::io::Write;
            if record.level() <= log::Level::Info {
                writeln!(buf, "{}", record.args())
            } else {
                writeln!(buf, "[{} {}] {}", record.level(), record.target(), record.args())
            }
        })
        .init();

    let cli = Cli::parse();
    let config = Config::load()?;

    orchestrator::run(cli.command, config)
}
