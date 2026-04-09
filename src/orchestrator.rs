use anyhow::Result;

use crate::traits::{AgentRunner, EventSink, IssueTracker, RemoteClient, RunConfig, VcsClient};

pub struct Orchestrator {
    issues: Box<dyn IssueTracker>,
    vcs: Box<dyn VcsClient>,
    remote: Box<dyn RemoteClient>,
    runner: Box<dyn AgentRunner>,
    events: Box<dyn EventSink>,
}

impl Orchestrator {
    pub fn new(
        issues: Box<dyn IssueTracker>,
        vcs: Box<dyn VcsClient>,
        remote: Box<dyn RemoteClient>,
        runner: Box<dyn AgentRunner>,
        events: Box<dyn EventSink>,
    ) -> Self {
        Self {
            issues,
            vcs,
            remote,
            runner,
            events,
        }
    }

    /// Implement a feature issue: resolve sub-issues in priority order,
    /// running a review/fix cycle after all work is complete.
    pub fn implement(&self, issue_id: u64, config: &RunConfig) -> Result<()> {
        todo!("fetch issue, identify children, prioritize, run work loop + review loop")
    }

    /// Work through all issues matching a label, one at a time.
    pub fn clear(&self, label: &str, config: &RunConfig) -> Result<()> {
        todo!("fetch issues by label, run work loop for each")
    }

    /// Run only the review phase against a completed issue.
    pub fn review(&self, issue_id: u64, config: &RunConfig) -> Result<()> {
        todo!("invoke review agent, create sub-issues for findings, re-run work loop if needed")
    }
}
