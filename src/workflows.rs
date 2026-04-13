use anyhow::Result;

use crate::behaviors::{complete_ticket, execute_ordered, interactive_config_wizard, scaffold_intern_directory, UserInteractor, WizardOutput};
use crate::cli::{Command, CommitStrategyArg};
use crate::config::Config;
use crate::context::Context;
use crate::git::GitClient;
use crate::github::GithubAdapter;
use crate::process::ProcessRunner;
use crate::reporter::log_reporter::LogReporter;
use crate::runner::LocalRunner;
use crate::traits::{CommitStrategy, RunConfig};

pub fn init_workflow(base_dir: &std::path::Path, interactor: &dyn UserInteractor) -> Result<()> {
    let wizard_output = interactive_config_wizard(base_dir, interactor)?;
    scaffold_intern_directory(base_dir, &wizard_output)
}

pub fn init_workflow_with_defaults(base_dir: &std::path::Path) -> Result<()> {
    scaffold_intern_directory(base_dir, &WizardOutput::defaults())
}

pub fn implement_workflow(issue_id: u64, ctx: &Context) -> Result<()> {
    complete_ticket(issue_id, ctx)
}

pub fn clear_workflow(label: &str, ctx: &Context) -> Result<()> {
    let issues = ctx.issues.get_issues_by_label(label)?;
    execute_ordered(&issues, ctx)
}

pub fn build_context(command: &Command, config: &Config) -> Result<Context> {
    let issues = match config.issue_tracker.kind.as_str() {
        "github" => Box::new(GithubAdapter::new(
            &config.issue_tracker.repo,
            Box::new(ProcessRunner),
        )) as Box<dyn crate::traits::IssueTracker>,
        kind => anyhow::bail!("unknown issue_tracker.kind: {kind}"),
    };

    let source_control = Box::new(GitClient::new(Box::new(ProcessRunner))) as Box<dyn crate::traits::SourceControl>;

    let remote = match config.issue_tracker.kind.as_str() {
        "github" => Box::new(GithubAdapter::new(
            &config.issue_tracker.repo,
            Box::new(ProcessRunner),
        )) as Box<dyn crate::traits::RemoteClient>,
        kind => anyhow::bail!("unknown remote kind: {kind}"),
    };

    let runner = match config.agent.kind.as_str() {
        "local" => Box::new(LocalRunner::new(
            Box::new(ProcessRunner),
            config.agent.settings_file.clone(),
        )) as Box<dyn crate::traits::AgentRunner>,
        kind => anyhow::bail!("unknown agent.kind: {kind}"),
    };

    let events = Box::new(LogReporter) as Box<dyn crate::traits::EventSink>;

    Ok(Context::new(issues, source_control, remote, runner, events, build_run_config(command, config)?))
}

pub fn build_run_config(command: &Command, config: &Config) -> Result<RunConfig> {
    let (dry_run, max_iterations_override, commit_strategy_override) = match command {
        Command::Implement { dry_run, max_iterations, commit_strategy, .. } => {
            (*dry_run, *max_iterations, commit_strategy.clone())
        }
        Command::Clear { dry_run, max_iterations, commit_strategy, .. } => {
            (*dry_run, *max_iterations, commit_strategy.clone())
        }
        Command::Review { dry_run, .. } => (*dry_run, None, None),
        Command::Init { .. } => unreachable!(),
    };

    let commit_strategy = match commit_strategy_override {
        Some(CommitStrategyArg::Direct) => CommitStrategy::Direct,
        Some(CommitStrategyArg::PerTicket) => CommitStrategy::PerTicket,
        Some(CommitStrategyArg::FeatureBranch) => CommitStrategy::FeatureBranch,
        None => CommitStrategy::from_key(&config.run.commit_strategy)
            .unwrap_or(CommitStrategy::FeatureBranch),
    };

    Ok(RunConfig {
        max_iterations: max_iterations_override.unwrap_or(config.run.max_iterations),
        commit_strategy,
        dry_run,
        repo_context: config.resolve_repo_context()?,
        work_directory: config.resolve_work_directory(),
    })
}
