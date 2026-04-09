use anyhow::Result;
use intern::traits::{
    AgentOutput, AgentRunner, Event, EventSink, Issue, IssueTracker, RemoteClient, RunConfig,
    VcsClient,
};

// --- Fake adapters for testing ---
// These implement the traits with predictable, controllable behavior
// so tests can exercise orchestrator logic without real GitHub/git/Claude.

struct FakeIssueTracker;
impl IssueTracker for FakeIssueTracker {
    fn get_issue(&self, _id: u64) -> Result<Issue> {
        todo!()
    }
    fn get_children(&self, _id: u64) -> Result<Vec<Issue>> {
        Ok(vec![])
    }
    fn get_issues_by_label(&self, _label: &str) -> Result<Vec<Issue>> {
        Ok(vec![])
    }
    fn claim_issue(&self, _id: u64) -> Result<()> {
        Ok(())
    }
    fn complete_issue(&self, _id: u64) -> Result<()> {
        Ok(())
    }
    fn skip_issue(&self, _id: u64) -> Result<()> {
        Ok(())
    }
    fn post_comment(&self, _id: u64, _body: &str) -> Result<()> {
        Ok(())
    }
    fn create_child_issue(&self, _parent_id: u64, _title: &str, _body: &str) -> Result<Issue> {
        todo!()
    }
}

struct FakeVcsClient;
impl VcsClient for FakeVcsClient {
    fn create_branch(&self, _name: &str) -> Result<()> {
        Ok(())
    }
    fn current_branch(&self) -> Result<String> {
        Ok("main".to_string())
    }
    fn diff_from_main(&self) -> Result<String> {
        Ok(String::new())
    }
    fn stage(&self, _paths: Option<&[&str]>) -> Result<()> {
        Ok(())
    }
    fn commit(&self, _message: &str) -> Result<()> {
        Ok(())
    }
}

struct FakeRemoteClient;
impl RemoteClient for FakeRemoteClient {
    fn create_pr(&self, _title: &str, _body: &str, _branch: &str) -> Result<String> {
        Ok("https://github.com/example/repo/pull/1".to_string())
    }
}

struct FakeRunner;
impl AgentRunner for FakeRunner {
    fn run(&self, _prompt: &str, _config: &RunConfig) -> Result<AgentOutput> {
        Ok(AgentOutput {
            stdout: String::new(),
            success: true,
        })
    }
}

struct FakeEventSink;
impl EventSink for FakeEventSink {
    fn emit(&self, _event: Event) {}
}

// --- Tests ---

#[test]
fn test_placeholder() {
    // Tests will go here once orchestrator logic is implemented.
    // Use the fake adapters above to construct an Orchestrator and drive it.
}
