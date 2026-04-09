use anyhow::Result;

use crate::traits::{
    AgentRunner, Event, EventSink, Issue, IssueTracker, RemoteClient, RunConfig, SourceControl,
};

pub struct Orchestrator {
    issues: Box<dyn IssueTracker>,
    vcs: Box<dyn SourceControl>,
    remote: Box<dyn RemoteClient>,
    runner: Box<dyn AgentRunner>,
    events: Box<dyn EventSink>,
}

impl Orchestrator {
    pub fn new(
        issues: Box<dyn IssueTracker>,
        vcs: Box<dyn SourceControl>,
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

    /// Implement a single issue: claim it, run the agent, mark complete on success.
    /// Skips issues labeled `hitl`.
    pub fn implement(&self, issue_id: u64, config: &RunConfig) -> Result<()> {
        let issue = self.issues.get_issue(issue_id)?;

        if issue.labels.contains(&"hitl".to_string()) {
            log::info!("skipping issue #{issue_id} — labeled hitl");
            return Ok(());
        }

        self.issues.claim_issue(issue_id)?;
        self.events.emit(Event::AgentStarted(issue_id));

        let prompt = build_implement_prompt(&issue);
        let output = self.runner.run(&prompt, config)?;

        if output.success {
            self.issues.complete_issue(issue_id)?;
            self.events.emit(Event::IssueComplete(issue_id));
        }

        self.events.emit(Event::RunComplete);
        Ok(())
    }

    /// Work through all issues matching a label, one at a time.
    pub fn clear(&self, _label: &str, _config: &RunConfig) -> Result<()> {
        todo!("fetch issues by label, run work loop for each")
    }

    /// Run only the review phase against a completed issue.
    pub fn review(&self, _issue_id: u64, _config: &RunConfig) -> Result<()> {
        todo!("invoke review agent, create sub-issues for findings, re-run work loop if needed")
    }
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
