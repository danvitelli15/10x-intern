use anyhow::Result;
use clap::Parser;

use intern::cli::{Cli, Command};
use intern::config::Config;
use intern::git::GitClient;
use intern::github::GithubAdapter;
use intern::orchestrator::Orchestrator;
use intern::process::ProcessRunner;
use intern::reporter::log_reporter::LogReporter;
use intern::runner::LocalRunner;
use intern::traits::{
    AgentRunner, CommitStrategy, EventSink, IssueTracker, RemoteClient, RunConfig, SourceControl,
};

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let config = Config::load()?;

    // Wire adapters from config — all driven by Box<dyn Trait> so swapping is a one-liner.
    let issues: Box<dyn IssueTracker> = match config.issue_tracker.kind.as_str() {
        "github" => Box::new(GithubAdapter::new(
            &config.issue_tracker.repo,
            Box::new(ProcessRunner),
        )),
        kind => anyhow::bail!("unknown issue_tracker.kind: {kind}"),
    };

    let vcs: Box<dyn SourceControl> = Box::new(GitClient::new(Box::new(ProcessRunner)));

    let remote: Box<dyn RemoteClient> = match config.issue_tracker.kind.as_str() {
        "github" => Box::new(GithubAdapter::new(
            &config.issue_tracker.repo,
            Box::new(ProcessRunner),
        )),
        kind => anyhow::bail!("unknown remote kind: {kind}"),
    };

    let agent: Box<dyn AgentRunner> = match config.agent.kind.as_str() {
        "local" => Box::new(LocalRunner::new(
            Box::new(ProcessRunner),
            config.agent.settings_file.clone(),
        )),
        kind => anyhow::bail!("unknown agent.kind: {kind}"),
    };

    let events: Box<dyn EventSink> = Box::new(LogReporter);

    let orchestrator = Orchestrator::new(issues, vcs, remote, agent, events);

    let commit_strategy = match config.run.commit_strategy.as_str() {
        "direct" => CommitStrategy::Direct,
        "per-ticket" => CommitStrategy::PerTicket,
        _ => CommitStrategy::FeatureBranch,
    };

    let base_config = RunConfig {
        max_iterations: config.run.max_iterations,
        commit_strategy,
        dry_run: false,
    };

    match cli.command {
        Command::Implement {
            issue_id,
            dry_run,
            max_iterations,
            ..
        } => {
            let config = RunConfig {
                dry_run,
                max_iterations: max_iterations.unwrap_or(base_config.max_iterations),
                ..base_config
            };
            orchestrator.implement(issue_id, &config)
        }
        Command::Clear {
            label,
            dry_run,
            max_iterations,
            ..
        } => {
            let config = RunConfig {
                dry_run,
                max_iterations: max_iterations.unwrap_or(base_config.max_iterations),
                ..base_config
            };
            orchestrator.clear(&label, &config)
        }
        Command::Review { issue_id, dry_run } => {
            let config = RunConfig {
                dry_run,
                ..base_config
            };
            orchestrator.review(issue_id, &config)
        }
    }
}
