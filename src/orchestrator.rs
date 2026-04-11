use anyhow::Result;

use crate::cli::{Command, CommitStrategyArg};
use crate::config::Config;
use crate::git::GitClient;
use crate::github::GithubAdapter;
use crate::process::ProcessRunner;
use crate::reporter::log_reporter::LogReporter;
use crate::runner::LocalRunner;
use crate::traits::{
    AgentRunner, CommitStrategy, Event, EventSink, Issue, IssueTracker, RemoteClient, RunConfig,
    SourceControl,
};

pub struct Context {
    pub issues: Box<dyn IssueTracker>,
    pub source_control: Box<dyn SourceControl>,
    pub remote: Box<dyn RemoteClient>,
    pub runner: Box<dyn AgentRunner>,
    pub events: Box<dyn EventSink>,
    pub config: RunConfig,
}

pub fn run(command: Command, config: Config) -> Result<()> {
    let ctx = build_context(&command, &config)?;
    match command {
        Command::Implement { issue_id, .. } => implement(issue_id, &ctx),
        Command::Clear { .. } => todo!("complete_series"),
        Command::Review { .. } => todo!("review"),
    }
}

fn build_context(command: &Command, config: &Config) -> Result<Context> {
    let issues: Box<dyn IssueTracker> = match config.issue_tracker.kind.as_str() {
        "github" => Box::new(GithubAdapter::new(
            &config.issue_tracker.repo,
            Box::new(ProcessRunner),
        )),
        kind => anyhow::bail!("unknown issue_tracker.kind: {kind}"),
    };

    let source_control: Box<dyn SourceControl> = Box::new(GitClient::new(Box::new(ProcessRunner)));

    let remote: Box<dyn RemoteClient> = match config.issue_tracker.kind.as_str() {
        "github" => Box::new(GithubAdapter::new(
            &config.issue_tracker.repo,
            Box::new(ProcessRunner),
        )),
        kind => anyhow::bail!("unknown remote kind: {kind}"),
    };

    let runner: Box<dyn AgentRunner> = match config.agent.kind.as_str() {
        "local" => Box::new(LocalRunner::new(
            Box::new(ProcessRunner),
            config.agent.settings_file.clone(),
        )),
        kind => anyhow::bail!("unknown agent.kind: {kind}"),
    };

    let events: Box<dyn EventSink> = Box::new(LogReporter);

    Ok(Context {
        issues,
        source_control,
        remote,
        runner,
        events,
        config: build_run_config(command, config),
    })
}

fn build_run_config(command: &Command, config: &Config) -> RunConfig {
    let (dry_run, max_iterations_override, commit_strategy_override) = match command {
        Command::Implement {
            dry_run,
            max_iterations,
            commit_strategy,
            ..
        } => (*dry_run, *max_iterations, commit_strategy.clone()),
        Command::Clear {
            dry_run,
            max_iterations,
            commit_strategy,
            ..
        } => (*dry_run, *max_iterations, commit_strategy.clone()),
        Command::Review { dry_run, .. } => (*dry_run, None, None),
    };

    let commit_strategy = match commit_strategy_override {
        Some(CommitStrategyArg::Direct) => CommitStrategy::Direct,
        Some(CommitStrategyArg::PerTicket) => CommitStrategy::PerTicket,
        Some(CommitStrategyArg::FeatureBranch) => CommitStrategy::FeatureBranch,
        None => match config.run.commit_strategy.as_str() {
            "direct" => CommitStrategy::Direct,
            "per-ticket" => CommitStrategy::PerTicket,
            _ => CommitStrategy::FeatureBranch,
        },
    };

    RunConfig {
        max_iterations: max_iterations_override.unwrap_or(config.run.max_iterations),
        commit_strategy,
        dry_run,
    }
}

pub fn implement(issue_id: u64, ctx: &Context) -> Result<()> {
    let issue = ctx.issues.get_issue(issue_id)?;

    if issue.labels.contains(&"hitl".to_string()) {
        log::info!("skipping issue #{issue_id} — labeled hitl");
        return Ok(());
    }

    ctx.issues.claim_issue(issue_id)?;
    ctx.events.emit(Event::AgentStarted(issue_id));

    let prompt = build_implement_prompt(&issue);
    let output = ctx.runner.run(&prompt, &ctx.config)?;

    if output.success {
        ctx.issues.complete_issue(issue_id)?;
        ctx.events.emit(Event::IssueComplete(issue_id));
    }

    ctx.events.emit(Event::RunComplete);
    Ok(())
}

/// Build the prompt for the implement agent.
///
/// Uses the embedded default prompt template. Variables injected: `{{issue_id}}`,
/// `{{issue_title}}`, `{{issue_body}}`.
///
/// TODO: check for a repo-local override at `.intern/prompts/implement.md` before
/// falling back to the embedded default. See issue #1.
fn build_implement_prompt(issue: &Issue) -> String {
    const DEFAULT: &str = include_str!("../prompts/implement.md");
    DEFAULT
        .replace("{{issue_id}}", &issue.id.to_string())
        .replace("{{issue_title}}", &issue.title)
        .replace("{{issue_body}}", &issue.body)
}
