#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use anyhow::Result;
use intern::behaviors::UserInteractor;
use intern::context::Context;
use intern::traits::{
    AgentOutput, AgentRunner, DirtyBehavior, Event, EventSink, Issue, IssueTracker, IssueType,
    MergeStrategy, RemoteClient, RunConfig, SourceControl,
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

    fn prompt_choice(&self, _question: &str, choices: &[String], default_idx: Option<usize>) -> Result<usize> {
        let idx = self.choice_responses.borrow_mut().pop_front()
            .or(default_idx)
            .expect("StubInteractor: no choice response queued and no default provided");
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
    fn create_branch(&self, _name: &str, _from: &str) -> Result<()> { Ok(()) }
    fn current_branch(&self) -> Result<String> { Ok("main".to_string()) }
    fn diff_from_base(&self, _base: &str) -> Result<String> { Ok(String::new()) }
    fn has_uncommitted_changes(&self) -> Result<bool> { Ok(false) }
    fn has_commits_since(&self, _: &str) -> Result<bool> { Ok(true) }
    fn stage(&self, _paths: Option<&[&str]>) -> Result<()> { Ok(()) }
    fn commit(&self, _message: &str) -> Result<()> { Ok(()) }
}

/// Records create_branch, stage, and commit calls.
/// Returns configurable values for current_branch, has_uncommitted_changes, has_commits_since.
/// Use recording_source_control() for the default happy-path configuration.
pub struct RecordingSourceControl {
    pub branches_created: Rc<RefCell<Vec<(String, String)>>>,
    pub commits_made: Rc<RefCell<Vec<String>>>,
    pub stages_made: Rc<RefCell<Vec<Option<Vec<String>>>>>,
    pub current_branch_result: String,
    pub uncommitted_changes: bool,
    pub has_commits: bool,
}

pub fn recording_source_control(current_branch: &str) -> (RecordingSourceControl, Rc<RefCell<Vec<(String, String)>>>) {
    let branches = Rc::new(RefCell::new(vec![]));
    let sc = RecordingSourceControl {
        branches_created: branches.clone(),
        commits_made: Rc::new(RefCell::new(vec![])),
        stages_made: Rc::new(RefCell::new(vec![])),
        current_branch_result: current_branch.to_string(),
        uncommitted_changes: false,
        has_commits: true,
    };
    (sc, branches)
}

/// Build a RecordingSourceControl with full control over all state.
pub fn recording_source_control_full(
    current_branch: &str,
    uncommitted_changes: bool,
    has_commits: bool,
) -> (RecordingSourceControl, Rc<RefCell<Vec<String>>>, Rc<RefCell<Vec<Option<Vec<String>>>>>) {
    let commits = Rc::new(RefCell::new(vec![]));
    let stages = Rc::new(RefCell::new(vec![]));
    let sc = RecordingSourceControl {
        branches_created: Rc::new(RefCell::new(vec![])),
        commits_made: commits.clone(),
        stages_made: stages.clone(),
        current_branch_result: current_branch.to_string(),
        uncommitted_changes,
        has_commits,
    };
    (sc, commits, stages)
}

impl SourceControl for RecordingSourceControl {
    fn create_branch(&self, name: &str, from: &str) -> Result<()> {
        self.branches_created.borrow_mut().push((name.to_string(), from.to_string()));
        Ok(())
    }
    fn current_branch(&self) -> Result<String> { Ok(self.current_branch_result.clone()) }
    fn diff_from_base(&self, _: &str) -> Result<String> { Ok(String::new()) }
    fn has_uncommitted_changes(&self) -> Result<bool> { Ok(self.uncommitted_changes) }
    fn has_commits_since(&self, _: &str) -> Result<bool> { Ok(self.has_commits) }
    fn stage(&self, paths: Option<&[&str]>) -> Result<()> {
        self.stages_made.borrow_mut().push(paths.map(|p| p.iter().map(|s| s.to_string()).collect()));
        Ok(())
    }
    fn commit(&self, message: &str) -> Result<()> {
        self.commits_made.borrow_mut().push(message.to_string());
        Ok(())
    }
}

pub struct FakeRemoteClient;
impl RemoteClient for FakeRemoteClient {
    fn create_pr(&self, _title: &str, _body: &str, _branch: &str) -> Result<String> {
        Ok("https://github.com/example/repo/pull/1".to_string())
    }
}

pub struct RecordingRemoteClient {
    pub calls: Rc<RefCell<Vec<String>>>,
}

impl RecordingRemoteClient {
    pub fn new() -> (Self, Rc<RefCell<Vec<String>>>) {
        let calls = Rc::new(RefCell::new(vec![]));
        (Self { calls: calls.clone() }, calls)
    }
}

impl RemoteClient for RecordingRemoteClient {
    fn create_pr(&self, _title: &str, _body: &str, branch: &str) -> Result<String> {
        self.calls.borrow_mut().push(branch.to_string());
        Ok(format!("https://github.com/example/repo/pull/{}", self.calls.borrow().len()))
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

// --- Sequenced command runner ---

pub struct SequencedCommandRunner {
    responses: RefCell<VecDeque<Result<String>>>,
}

impl SequencedCommandRunner {
    pub fn new() -> Self {
        Self { responses: RefCell::new(VecDeque::new()) }
    }

    pub fn then_ok(self, output: &str) -> Self {
        self.responses.borrow_mut().push_back(Ok(output.to_string()));
        self
    }

    pub fn then_err(self, msg: &str) -> Self {
        self.responses.borrow_mut().push_back(Err(anyhow::anyhow!("{}", msg)));
        self
    }
}

impl intern::traits::CommandRunner for SequencedCommandRunner {
    fn run(&self, _program: &str, _args: &[&str]) -> Result<String> {
        self.responses.borrow_mut().pop_front()
            .unwrap_or_else(|| Err(anyhow::anyhow!("SequencedCommandRunner: no more responses")))
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
        merge_strategy: MergeStrategy::Direct,
        base_branch: "main".to_string(),
        use_worktree: false,
        on_dirty_after_commit: DirtyBehavior::Warn,
        on_dirty_no_commits: DirtyBehavior::Fail,
        dry_run: false,
        repo_context: String::new(),
        work_directory: dir.path().to_path_buf(),
    }
}

pub fn run_config_with_strategy(dir: &tempfile::TempDir, strategy: MergeStrategy, base_branch: &str) -> RunConfig {
    RunConfig {
        max_iterations: 10,
        merge_strategy: strategy,
        base_branch: base_branch.to_string(),
        use_worktree: false,
        on_dirty_after_commit: DirtyBehavior::Warn,
        on_dirty_no_commits: DirtyBehavior::Fail,
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
        RunConfig { max_iterations, merge_strategy: MergeStrategy::Direct, base_branch: "main".to_string(), use_worktree: false, on_dirty_after_commit: DirtyBehavior::Warn, on_dirty_no_commits: DirtyBehavior::Fail, dry_run: false, repo_context: String::new(), work_directory: dir.path().to_path_buf() },
    );
    (ctx, dir)
}
