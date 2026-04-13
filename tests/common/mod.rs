#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use anyhow::Result;
use intern::behaviors::UserInteractor;
use intern::context::Context;
use intern::traits::{
    AgentOutput, AgentRunner, CommitStrategy, Event, EventSink, Issue, IssueTracker, IssueType,
    RemoteClient, RunConfig, SourceControl,
};

// --- Stub interactor ---

pub struct StubInteractor {
    pub text_responses: RefCell<VecDeque<String>>,
    pub choice_responses: RefCell<VecDeque<usize>>,
    pub confirm_responses: RefCell<VecDeque<bool>>,
}

impl StubInteractor {
    pub fn new() -> Self {
        Self {
            text_responses: RefCell::new(VecDeque::new()),
            choice_responses: RefCell::new(VecDeque::new()),
            confirm_responses: RefCell::new(VecDeque::new()),
        }
    }

    pub fn with_text(self, response: &str) -> Self {
        self.text_responses.borrow_mut().push_back(response.to_string());
        self
    }

    pub fn with_choice(self, index: usize) -> Self {
        self.choice_responses.borrow_mut().push_back(index);
        self
    }

    pub fn with_confirm(self, response: bool) -> Self {
        self.confirm_responses.borrow_mut().push_back(response);
        self
    }
}

impl UserInteractor for StubInteractor {
    fn prompt_text(&self, _question: &str, default: Option<&str>) -> Result<String> {
        Ok(self.text_responses.borrow_mut().pop_front()
            .or_else(|| default.map(|s| s.to_string()))
            .expect("StubInteractor: no text response queued"))
    }

    fn prompt_choice(&self, _question: &str, choices: &[String]) -> Result<usize> {
        let idx = self.choice_responses.borrow_mut().pop_front()
            .expect("StubInteractor: no choice response queued");
        assert!(idx < choices.len(), "StubInteractor: choice index out of range");
        Ok(idx)
    }

    fn prompt_confirm(&self, _question: &str, default: bool) -> Result<bool> {
        Ok(self.confirm_responses.borrow_mut().pop_front().unwrap_or(default))
    }
}

// --- Fake issue tracker ---

pub type ChildrenHandle = Rc<RefCell<VecDeque<Vec<Issue>>>>;

pub struct FakeIssueTracker {
    pub issues: Vec<Issue>,
    pub children_calls: ChildrenHandle,
    pub claimed: Rc<RefCell<Vec<u64>>>,
    pub completed: Rc<RefCell<Vec<u64>>>,
    pub skipped: Rc<RefCell<Vec<u64>>>,
}

impl FakeIssueTracker {
    pub fn new(issue: Issue) -> (Self, Rc<RefCell<Vec<u64>>>, Rc<RefCell<Vec<u64>>>, Rc<RefCell<Vec<u64>>>, ChildrenHandle) {
        Self::with_issues(vec![issue])
    }

    pub fn with_issues(issues: Vec<Issue>) -> (Self, Rc<RefCell<Vec<u64>>>, Rc<RefCell<Vec<u64>>>, Rc<RefCell<Vec<u64>>>, ChildrenHandle) {
        let claimed = Rc::new(RefCell::new(vec![]));
        let completed = Rc::new(RefCell::new(vec![]));
        let skipped = Rc::new(RefCell::new(vec![]));
        let children_calls: ChildrenHandle = Rc::new(RefCell::new(VecDeque::new()));
        (
            Self { issues, children_calls: children_calls.clone(), claimed: claimed.clone(), completed: completed.clone(), skipped: skipped.clone() },
            claimed, completed, skipped, children_calls,
        )
    }
}

impl IssueTracker for FakeIssueTracker {
    fn get_issue(&self, id: u64) -> Result<Issue> {
        self.issues.iter().find(|i| i.id == id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("issue {id} not found in fake"))
    }
    fn get_children(&self, _id: u64) -> Result<Vec<Issue>> {
        Ok(self.children_calls.borrow_mut().pop_front().unwrap_or_default())
    }
    fn get_issues_by_label(&self, _label: &str) -> Result<Vec<Issue>> {
        Ok(self.issues.clone())
    }
    fn claim_issue(&self, id: u64) -> Result<()> {
        self.claimed.borrow_mut().push(id);
        Ok(())
    }
    fn complete_issue(&self, id: u64) -> Result<()> {
        self.completed.borrow_mut().push(id);
        Ok(())
    }
    fn skip_issue(&self, id: u64) -> Result<()> {
        self.skipped.borrow_mut().push(id);
        Ok(())
    }
    fn post_comment(&self, _id: u64, _body: &str) -> Result<()> { Ok(()) }
    fn create_child_issue(&self, _parent_id: u64, _title: &str, _body: &str) -> Result<Issue> { todo!() }
    fn issue_type(&self, _id: u64) -> Result<IssueType> { Ok(IssueType::Ticket) }
}

// --- Other fakes ---

pub struct FakeSourceControl;
impl SourceControl for FakeSourceControl {
    fn create_branch(&self, _name: &str) -> Result<()> { Ok(()) }
    fn current_branch(&self) -> Result<String> { Ok("main".to_string()) }
    fn diff_from_base(&self, _base: &str) -> Result<String> { Ok(String::new()) }
    fn stage(&self, _paths: Option<&[&str]>) -> Result<()> { Ok(()) }
    fn commit(&self, _message: &str) -> Result<()> { Ok(()) }
}

pub struct FakeRemoteClient;
impl RemoteClient for FakeRemoteClient {
    fn create_pr(&self, _title: &str, _body: &str, _branch: &str) -> Result<String> {
        Ok("https://github.com/example/repo/pull/1".to_string())
    }
}

pub struct FakeEventSink;
impl EventSink for FakeEventSink {
    fn emit(&self, _event: Event) {}
}

pub struct FakeRunner {
    pub success: bool,
    pub prompt_received: Rc<RefCell<Option<String>>>,
}

impl FakeRunner {
    pub fn succeeds() -> (Self, Rc<RefCell<Option<String>>>) {
        let prompt = Rc::new(RefCell::new(None));
        (Self { success: true, prompt_received: prompt.clone() }, prompt)
    }
    pub fn fails() -> Self {
        Self { success: false, prompt_received: Rc::new(RefCell::new(None)) }
    }
}

impl AgentRunner for FakeRunner {
    fn run(&self, prompt: &str, _config: &RunConfig) -> Result<AgentOutput> {
        *self.prompt_received.borrow_mut() = Some(prompt.to_string());
        Ok(AgentOutput { stdout: String::new(), success: self.success })
    }
}

pub struct SequencedRunner {
    pub responses: RefCell<VecDeque<AgentOutput>>,
}

impl SequencedRunner {
    pub fn new(responses: Vec<AgentOutput>) -> Self {
        Self { responses: RefCell::new(responses.into()) }
    }
}

impl AgentRunner for SequencedRunner {
    fn run(&self, _prompt: &str, _config: &RunConfig) -> Result<AgentOutput> {
        self.responses.borrow_mut().pop_front()
            .ok_or_else(|| anyhow::anyhow!("SequencedRunner: no more responses queued"))
    }
}

// --- Helpers ---

pub fn make_issue(id: u64, labels: Vec<&str>) -> Issue {
    Issue {
        id,
        title: format!("Issue {id}"),
        body: format!("Body of issue {id}"),
        labels: labels.into_iter().map(|s| s.to_string()).collect(),
    }
}

pub fn agent_success(stdout: &str) -> AgentOutput {
    AgentOutput { stdout: stdout.to_string(), success: true }
}

pub fn make_prompts_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let prompts_dir = dir.path().join(".intern/prompts");
    std::fs::create_dir_all(&prompts_dir).unwrap();
    for name in &["implement", "review", "feature_review", "plan_order", "test_instructions"] {
        std::fs::write(prompts_dir.join(format!("{name}.md")), "{{issue_id}} {{issue_title}} {{issue_body}} {{diff}} {{issues_list}} {{repo_context}}").unwrap();
    }
    dir
}

pub fn run_config_with_dir(dir: &tempfile::TempDir) -> RunConfig {
    RunConfig {
        max_iterations: 10,
        commit_strategy: CommitStrategy::Direct,
        dry_run: false,
        repo_context: String::new(),
        work_directory: dir.path().to_path_buf(),
    }
}

pub fn make_context(tracker: FakeIssueTracker, runner: FakeRunner) -> (Context, tempfile::TempDir) {
    let dir = make_prompts_dir();
    let ctx = Context::new(
        Box::new(tracker),
        Box::new(FakeSourceControl),
        Box::new(FakeRemoteClient),
        Box::new(runner),
        Box::new(FakeEventSink),
        run_config_with_dir(&dir),
    );
    (ctx, dir)
}

pub fn make_context_sequenced(tracker: FakeIssueTracker, runner: SequencedRunner, max_iterations: u32) -> (Context, tempfile::TempDir) {
    let dir = make_prompts_dir();
    let ctx = Context::new(
        Box::new(tracker),
        Box::new(FakeSourceControl),
        Box::new(FakeRemoteClient),
        Box::new(runner),
        Box::new(FakeEventSink),
        RunConfig { max_iterations, commit_strategy: CommitStrategy::Direct, dry_run: false, repo_context: String::new(), work_directory: dir.path().to_path_buf() },
    );
    (ctx, dir)
}
