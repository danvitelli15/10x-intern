use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use intern::orchestrator::Orchestrator;
use intern::traits::{
    AgentOutput, AgentRunner, CommitStrategy, Event, EventSink, Issue, IssueTracker, RemoteClient,
    RunConfig, SourceControl,
};

// --- Helpers ---

fn make_issue(id: u64, labels: Vec<&str>) -> Issue {
    Issue {
        id,
        title: format!("Issue {id}"),
        body: format!("Body of issue {id}"),
        labels: labels.into_iter().map(|s| s.to_string()).collect(),
    }
}

fn run_config() -> RunConfig {
    RunConfig {
        max_iterations: 10,
        commit_strategy: CommitStrategy::Direct,
        dry_run: false,
    }
}

// --- Controllable fakes ---

struct FakeIssueTracker {
    issue: Issue,
    claimed: Rc<RefCell<Vec<u64>>>,
    completed: Rc<RefCell<Vec<u64>>>,
}

impl FakeIssueTracker {
    fn new(issue: Issue) -> (Self, Rc<RefCell<Vec<u64>>>, Rc<RefCell<Vec<u64>>>) {
        let claimed = Rc::new(RefCell::new(vec![]));
        let completed = Rc::new(RefCell::new(vec![]));
        (Self { issue, claimed: claimed.clone(), completed: completed.clone() }, claimed, completed)
    }
}

impl IssueTracker for FakeIssueTracker {
    fn get_issue(&self, _id: u64) -> Result<Issue> {
        Ok(self.issue.clone())
    }
    fn get_children(&self, _id: u64) -> Result<Vec<Issue>> {
        Ok(vec![])
    }
    fn get_issues_by_label(&self, _label: &str) -> Result<Vec<Issue>> {
        Ok(vec![])
    }
    fn claim_issue(&self, id: u64) -> Result<()> {
        self.claimed.borrow_mut().push(id);
        Ok(())
    }
    fn complete_issue(&self, id: u64) -> Result<()> {
        self.completed.borrow_mut().push(id);
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

struct FakeSourceControl;
impl SourceControl for FakeSourceControl {
    fn create_branch(&self, _name: &str) -> Result<()> { Ok(()) }
    fn current_branch(&self) -> Result<String> { Ok("main".to_string()) }
    fn diff_from_base(&self, _base: &str) -> Result<String> { Ok(String::new()) }
    fn stage(&self, _paths: Option<&[&str]>) -> Result<()> { Ok(()) }
    fn commit(&self, _message: &str) -> Result<()> { Ok(()) }
}

struct FakeRemoteClient;
impl RemoteClient for FakeRemoteClient {
    fn create_pr(&self, _title: &str, _body: &str, _branch: &str) -> Result<String> {
        Ok("https://github.com/example/repo/pull/1".to_string())
    }
}

struct FakeRunner {
    success: bool,
    prompt_received: Rc<RefCell<Option<String>>>,
}

impl FakeRunner {
    fn succeeds() -> (Self, Rc<RefCell<Option<String>>>) {
        let prompt = Rc::new(RefCell::new(None));
        (Self { success: true, prompt_received: prompt.clone() }, prompt)
    }
    fn fails() -> Self {
        Self { success: false, prompt_received: Rc::new(RefCell::new(None)) }
    }
}

impl AgentRunner for FakeRunner {
    fn run(&self, prompt: &str, _config: &RunConfig) -> Result<AgentOutput> {
        *self.prompt_received.borrow_mut() = Some(prompt.to_string());
        Ok(AgentOutput { stdout: String::new(), success: self.success })
    }
}

struct FakeEventSink;
impl EventSink for FakeEventSink {
    fn emit(&self, _event: Event) {}
}

fn orchestrator(tracker: FakeIssueTracker, runner: FakeRunner) -> Orchestrator {
    Orchestrator::new(
        Box::new(tracker),
        Box::new(FakeSourceControl),
        Box::new(FakeRemoteClient),
        Box::new(runner),
        Box::new(FakeEventSink),
    )
}

// --- Tests ---

#[test]
fn implement_claims_issue_before_running_agent() {
    let (tracker, claimed, _) = FakeIssueTracker::new(make_issue(42, vec![]));
    let (runner, _) = FakeRunner::succeeds();

    orchestrator(tracker, runner).implement(42, &run_config()).unwrap();

    assert!(claimed.borrow().contains(&42));
}

#[test]
fn implement_runs_agent_with_issue_content_in_prompt() {
    let (tracker, _, _) = FakeIssueTracker::new(make_issue(42, vec![]));
    let (runner, prompt) = FakeRunner::succeeds();

    orchestrator(tracker, runner).implement(42, &run_config()).unwrap();

    let prompt = prompt.borrow();
    let prompt = prompt.as_ref().unwrap();
    assert!(prompt.contains("Issue 42"));
    assert!(prompt.contains("Body of issue 42"));
}

#[test]
fn implement_marks_complete_when_agent_succeeds() {
    let (tracker, _, completed) = FakeIssueTracker::new(make_issue(42, vec![]));
    let (runner, _) = FakeRunner::succeeds();

    orchestrator(tracker, runner).implement(42, &run_config()).unwrap();

    assert!(completed.borrow().contains(&42));
}

#[test]
fn implement_skips_hitl_issues_without_running_agent() {
    let (tracker, claimed, _) = FakeIssueTracker::new(make_issue(42, vec!["hitl"]));
    let (runner, prompt) = FakeRunner::succeeds();

    orchestrator(tracker, runner).implement(42, &run_config()).unwrap();

    assert!(claimed.borrow().is_empty());
    assert!(prompt.borrow().is_none());
}

#[test]
fn implement_does_not_mark_complete_when_agent_fails() {
    let (tracker, _, completed) = FakeIssueTracker::new(make_issue(42, vec![]));
    let runner = FakeRunner::fails();

    orchestrator(tracker, runner).implement(42, &run_config()).unwrap();

    assert!(completed.borrow().is_empty());
}
