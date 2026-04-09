use anyhow::Result;
use clap::Parser;

use intern::cli::{Cli, Command};
use intern::config::Config;

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let config = Config::load()?;

    // TODO: wire up adapters from config and construct orchestrator
    //
    // let issues: Box<dyn IssueTracker> = match config.issue_tracker.kind.as_str() {
    //     "github" => Box::new(GithubAdapter::new(&config.issue_tracker.repo)),
    //     kind => anyhow::bail!("unknown issue tracker: {}", kind),
    // };
    // let vcs = Box::new(GitClient::new("."));
    // let remote: Box<dyn RemoteClient> = Box::new(GithubAdapter::new(&config.issue_tracker.repo));
    // let runner = Box::new(LocalRunner::new(config.agent.settings_file.clone()));
    // let events = Box::new(LogReporter);
    // let orchestrator = Orchestrator::new(issues, vcs, remote, runner, events);

    match cli.command {
        Command::Implement { issue_id, .. } => {
            log::info!("implement issue #{}", issue_id);
            todo!("wire orchestrator and call orchestrator.implement()")
        }
        Command::Clear { label, .. } => {
            log::info!("clear label: {}", label);
            todo!("wire orchestrator and call orchestrator.clear()")
        }
        Command::Review { issue_id, .. } => {
            log::info!("review issue #{}", issue_id);
            todo!("wire orchestrator and call orchestrator.review()")
        }
    }
}
