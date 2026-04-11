use std::cell::Cell;

use anyhow::Result;

use crate::actions::{feature_review, generate_test_instructions, implement, plan_order, review};
use crate::cli::{Command, CommitStrategyArg};
use crate::config::Config;
use crate::git::GitClient;
use crate::github::GithubAdapter;
use crate::process::ProcessRunner;
use crate::reporter::log_reporter::LogReporter;
use crate::runner::LocalRunner;
use crate::traits::{
    AgentOutput, AgentRunner, CommitStrategy, EventSink, IssueTracker, IssueType, RemoteClient,
    RunConfig, SourceControl,
};

pub fn complete_series(label: &str, ctx: &Context) -> Result<()> {
    let issues = ctx.issues.get_issues_by_label(label)?;
    execute_ordered(&issues, ctx)
}

pub fn complete_feature(issue_id: u64, ctx: &Context) -> Result<()> {
    let initial_children = ctx.issues.get_children(issue_id)?;
    let initial_ids: std::collections::HashSet<u64> = initial_children.iter().map(|i| i.id).collect();
    execute_ordered(&initial_children, ctx)?;

    let has_findings = feature_review(issue_id, ctx)?;
    if has_findings {
        let all_children = ctx.issues.get_children(issue_id)?;
        let new_children: Vec<_> = all_children.into_iter().filter(|i| !initial_ids.contains(&i.id)).collect();
        execute_ordered(&new_children, ctx)?;

        if feature_review(issue_id, ctx)? {
            ctx.issues.skip_issue(issue_id)?;
            return Ok(());
        }
    }

    generate_test_instructions(issue_id, ctx)?;
    Ok(())
}

fn execute_ordered(issues: &[crate::traits::Issue], ctx: &Context) -> Result<()> {
    let ordered_ids = plan_order(issues, ctx)?;
    for id in ordered_ids {
        match ctx.issues.issue_type(id)? {
            IssueType::Ticket => complete_ticket(id, ctx)?,
            IssueType::Feature => complete_feature(id, ctx)?,
        }
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) struct BudgetExhausted;

impl std::fmt::Display for BudgetExhausted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "agent budget exhausted")
    }
}

impl std::error::Error for BudgetExhausted {}

pub struct Context {
    pub issues: Box<dyn IssueTracker>,
    pub source_control: Box<dyn SourceControl>,
    pub remote: Box<dyn RemoteClient>,
    pub events: Box<dyn EventSink>,
    pub config: RunConfig,
    runner: Box<dyn AgentRunner>,
    iterations_used: Cell<u32>,
}

impl Context {
    pub fn new(
        issues: Box<dyn IssueTracker>,
        source_control: Box<dyn SourceControl>,
        remote: Box<dyn RemoteClient>,
        runner: Box<dyn AgentRunner>,
        events: Box<dyn EventSink>,
        config: RunConfig,
    ) -> Self {
        Self {
            issues,
            source_control,
            remote,
            runner,
            events,
            config,
            iterations_used: Cell::new(0),
        }
    }

    pub fn run_agent(&self, prompt: &str) -> Result<AgentOutput> {
        let used = self.iterations_used.get();
        if used >= self.config.max_iterations {
            return Err(anyhow::anyhow!(BudgetExhausted));
        }
        self.iterations_used.set(used + 1);
        self.runner.run(prompt, &self.config)
    }

    pub fn iterations_used(&self) -> u32 {
        self.iterations_used.get()
    }
}

pub fn run(command: Command, config: Config) -> Result<()> {
    let ctx = build_context(&command, &config)?;
    match command {
        Command::Implement { issue_id, .. } => complete_ticket(issue_id, &ctx),
        Command::Clear { label, .. } => complete_series(&label, &ctx),
        Command::Review { .. } => todo!("review"),
    }
}

pub fn complete_ticket(issue_id: u64, ctx: &Context) -> Result<()> {
    loop {
        let result = (|| -> Result<bool> {
            implement(issue_id, ctx)?;
            review(issue_id, ctx)
        })();

        match result {
            Ok(false) => break,
            Ok(true) => continue,
            Err(e) if e.downcast_ref::<BudgetExhausted>().is_some() => {
                ctx.issues.skip_issue(issue_id)?;
                return Ok(());
            }
            Err(e) => return Err(e),
        }
    }
    generate_test_instructions(issue_id, ctx)?;
    Ok(())
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

    Ok(Context::new(issues, source_control, remote, runner, events, build_run_config(command, config)))
}

fn build_run_config(command: &Command, config: &Config) -> RunConfig {
    let (dry_run, max_iterations_override, commit_strategy_override) = match command {
        Command::Implement { dry_run, max_iterations, commit_strategy, .. } => {
            (*dry_run, *max_iterations, commit_strategy.clone())
        }
        Command::Clear { dry_run, max_iterations, commit_strategy, .. } => {
            (*dry_run, *max_iterations, commit_strategy.clone())
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{CommitStrategy, Event, Issue, RunConfig};

    struct StubIssueTracker;
    impl IssueTracker for StubIssueTracker {
        fn get_issue(&self, id: u64) -> Result<Issue> {
            Ok(Issue { id, title: "".into(), body: "".into(), labels: vec![] })
        }
        fn get_children(&self, _: u64) -> Result<Vec<Issue>> { Ok(vec![]) }
        fn get_issues_by_label(&self, _: &str) -> Result<Vec<Issue>> { Ok(vec![]) }
        fn claim_issue(&self, _: u64) -> Result<()> { Ok(()) }
        fn complete_issue(&self, _: u64) -> Result<()> { Ok(()) }
        fn skip_issue(&self, _: u64) -> Result<()> { Ok(()) }
        fn post_comment(&self, _: u64, _: &str) -> Result<()> { Ok(()) }
        fn create_child_issue(&self, _: u64, _: &str, _: &str) -> Result<Issue> { unimplemented!() }
        fn issue_type(&self, _: u64) -> Result<IssueType> { Ok(IssueType::Ticket) }
    }

    struct StubSourceControl;
    impl SourceControl for StubSourceControl {
        fn create_branch(&self, _: &str) -> Result<()> { Ok(()) }
        fn current_branch(&self) -> Result<String> { Ok("main".into()) }
        fn diff_from_base(&self, _: &str) -> Result<String> { Ok("".into()) }
        fn stage(&self, _: Option<&[&str]>) -> Result<()> { Ok(()) }
        fn commit(&self, _: &str) -> Result<()> { Ok(()) }
    }

    struct StubRemoteClient;
    impl RemoteClient for StubRemoteClient {
        fn create_pr(&self, _: &str, _: &str, _: &str) -> Result<String> { Ok("".into()) }
    }

    struct StubEventSink;
    impl EventSink for StubEventSink {
        fn emit(&self, _: Event) {}
    }

    struct StubRunner;
    impl AgentRunner for StubRunner {
        fn run(&self, _: &str, _: &RunConfig) -> Result<AgentOutput> {
            Ok(AgentOutput { stdout: "".into(), success: true })
        }
    }

    fn test_context(max_iterations: u32) -> Context {
        Context::new(
            Box::new(StubIssueTracker),
            Box::new(StubSourceControl),
            Box::new(StubRemoteClient),
            Box::new(StubRunner),
            Box::new(StubEventSink),
            RunConfig { max_iterations, commit_strategy: CommitStrategy::Direct, dry_run: false },
        )
    }

    #[test]
    fn context_starts_with_zero_iterations_used() {
        let ctx = test_context(10);
        assert_eq!(ctx.iterations_used(), 0);
    }

    #[test]
    fn run_agent_increments_iteration_count() {
        let ctx = test_context(10);
        ctx.run_agent("prompt").unwrap();
        assert_eq!(ctx.iterations_used(), 1);
    }

    #[test]
    fn run_agent_errors_when_max_iterations_reached() {
        let ctx = test_context(2);
        ctx.run_agent("p1").unwrap();
        ctx.run_agent("p2").unwrap();
        let result = ctx.run_agent("p3");
        assert!(result.is_err());
    }
}
