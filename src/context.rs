use std::cell::Cell;

use anyhow::Result;

use crate::traits::{AgentOutput, AgentRunner, EventSink, IssueTracker, RemoteClient, RunConfig, SourceControl};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{AgentOutput, AgentRunner, CommitStrategy, Event, EventSink, Issue,
        IssueTracker, IssueType, RemoteClient, RunConfig, SourceControl};

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
            RunConfig {
                max_iterations,
                commit_strategy: CommitStrategy::Direct,
                dry_run: false,
                repo_context: String::new(),
                work_directory: std::path::PathBuf::from("."),
            },
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
