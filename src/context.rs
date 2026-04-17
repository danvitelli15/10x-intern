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
        log::trace!("run_agent: iteration {}/{}", used + 1, self.config.max_iterations);
        if used >= self.config.max_iterations {
            log::debug!("run_agent: budget exhausted after {used} iteration(s)");
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
    use crate::test_utils::{StubEventSink, StubIssueTracker, StubRemoteClient, StubSourceControl};
    use crate::traits::{AgentOutput, AgentRunner, MergeStrategy, RunConfig};

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
                merge_strategy: MergeStrategy::Direct,
                base_branch: "main".to_string(),
                use_worktree: false,
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
