use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use anyhow::Result;
use intern::cli::Command;
use intern::config::{AgentConfig, Config, IssueTrackerConfig, RunDefaults};
use intern::actions::implement;
use intern::orchestrator::{complete_feature, complete_series, complete_ticket, run, Context};
use intern::traits::{
    AgentOutput, AgentRunner, CommitStrategy, Event, EventSink, Issue, IssueTracker, IssueType,
    RemoteClient, RunConfig, SourceControl,
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
        repo_context: String::new(),
    }
}

// --- Controllable fakes ---

type ChildrenHandle = Rc<RefCell<VecDeque<Vec<Issue>>>>;

struct FakeIssueTracker {
    issues: Vec<Issue>,
    children_calls: ChildrenHandle,
    claimed: Rc<RefCell<Vec<u64>>>,
    completed: Rc<RefCell<Vec<u64>>>,
    skipped: Rc<RefCell<Vec<u64>>>,
}

impl FakeIssueTracker {
    fn new(issue: Issue) -> (Self, Rc<RefCell<Vec<u64>>>, Rc<RefCell<Vec<u64>>>, Rc<RefCell<Vec<u64>>>, ChildrenHandle) {
        Self::with_issues(vec![issue])
    }

    fn with_issues(issues: Vec<Issue>) -> (Self, Rc<RefCell<Vec<u64>>>, Rc<RefCell<Vec<u64>>>, Rc<RefCell<Vec<u64>>>, ChildrenHandle) {
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
    fn post_comment(&self, _id: u64, _body: &str) -> Result<()> {
        Ok(())
    }
    fn create_child_issue(&self, _parent_id: u64, _title: &str, _body: &str) -> Result<Issue> {
        todo!()
    }
    fn issue_type(&self, _id: u64) -> Result<IssueType> {
        Ok(IssueType::Ticket)
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

struct SequencedRunner {
    responses: RefCell<VecDeque<AgentOutput>>,
}

impl SequencedRunner {
    fn new(responses: Vec<AgentOutput>) -> Self {
        Self { responses: RefCell::new(responses.into()) }
    }
}

impl AgentRunner for SequencedRunner {
    fn run(&self, _prompt: &str, _config: &RunConfig) -> Result<AgentOutput> {
        self.responses.borrow_mut().pop_front()
            .ok_or_else(|| anyhow::anyhow!("SequencedRunner: no more responses queued"))
    }
}

fn agent_success(stdout: &str) -> AgentOutput {
    AgentOutput { stdout: stdout.to_string(), success: true }
}

struct FakeEventSink;
impl EventSink for FakeEventSink {
    fn emit(&self, _event: Event) {}
}

fn make_context(tracker: FakeIssueTracker, runner: FakeRunner) -> Context {
    Context::new(
        Box::new(tracker),
        Box::new(FakeSourceControl),
        Box::new(FakeRemoteClient),
        Box::new(runner),
        Box::new(FakeEventSink),
        run_config(),
    )
}

fn make_context_sequenced(tracker: FakeIssueTracker, runner: SequencedRunner, max_iterations: u32) -> Context {
    Context::new(
        Box::new(tracker),
        Box::new(FakeSourceControl),
        Box::new(FakeRemoteClient),
        Box::new(runner),
        Box::new(FakeEventSink),
        RunConfig { max_iterations, commit_strategy: CommitStrategy::Direct, dry_run: false, repo_context: String::new() },
    )
}

// --- Tests (new interface) ---

#[test]
fn implement_fn_claims_issue_before_running_agent() {
    let (tracker, claimed, _, _, _) = FakeIssueTracker::new(make_issue(42, vec![]));
    let (runner, _) = FakeRunner::succeeds();

    implement(42, &make_context(tracker, runner)).unwrap();

    assert!(claimed.borrow().contains(&42));
}

#[test]
fn implement_fn_runs_agent_with_issue_content_in_prompt() {
    let (tracker, _, _, _, _) = FakeIssueTracker::new(make_issue(42, vec![]));
    let (runner, prompt) = FakeRunner::succeeds();

    implement(42, &make_context(tracker, runner)).unwrap();

    let prompt = prompt.borrow();
    let prompt = prompt.as_ref().unwrap();
    assert!(prompt.contains("Issue 42"));
    assert!(prompt.contains("Body of issue 42"));
}

#[test]
fn implement_fn_marks_complete_when_agent_succeeds() {
    let (tracker, _, completed, _, _) = FakeIssueTracker::new(make_issue(42, vec![]));
    let (runner, _) = FakeRunner::succeeds();

    implement(42, &make_context(tracker, runner)).unwrap();

    assert!(completed.borrow().contains(&42));
}

#[test]
fn implement_fn_skips_hitl_issues_without_running_agent() {
    let (tracker, claimed, _, _, _) = FakeIssueTracker::new(make_issue(42, vec!["hitl"]));
    let (runner, prompt) = FakeRunner::succeeds();

    implement(42, &make_context(tracker, runner)).unwrap();

    assert!(claimed.borrow().is_empty());
    assert!(prompt.borrow().is_none());
}

#[test]
fn implement_fn_does_not_mark_complete_when_agent_fails() {
    let (tracker, _, completed, _, _) = FakeIssueTracker::new(make_issue(42, vec![]));
    let runner = FakeRunner::fails();

    implement(42, &make_context(tracker, runner)).unwrap();

    assert!(completed.borrow().is_empty());
}

fn implement_command(issue_id: u64) -> Command {
    Command::Implement { issue_id, dry_run: false, max_iterations: None, commit_strategy: None }
}

fn github_config() -> Config {
    Config {
        issue_tracker: IssueTrackerConfig { kind: "github".to_string(), repo: "owner/repo".to_string() },
        agent: AgentConfig { kind: "local".to_string(), settings_file: None },
        run: RunDefaults::default(),
        context_file: None,
    }
}

#[test]
fn run_returns_error_for_unknown_issue_tracker_kind() {
    let mut config = github_config();
    config.issue_tracker.kind = "linear".to_string();

    let result = run(implement_command(1), config);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("linear"));
}

#[test]
fn run_returns_error_for_unknown_agent_kind() {
    let mut config = github_config();
    config.agent.kind = "docker".to_string();

    let result = run(implement_command(1), config);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("docker"));
}

// --- complete_ticket tests ---

#[test]
fn complete_ticket_runs_implement_review_and_instructions_when_clean() {
    let (tracker, _, _, _, _) = FakeIssueTracker::new(make_issue(42, vec![]));
    let runner = SequencedRunner::new(vec![
        agent_success(""),        // implement
        agent_success("CLEAN"),   // review
        agent_success(""),        // generate_test_instructions
    ]);
    let ctx = make_context_sequenced(tracker, runner, 10);

    complete_ticket(42, &ctx).unwrap();

    assert_eq!(ctx.iterations_used(), 3);
}

#[test]
fn complete_ticket_loops_when_review_has_findings() {
    let (tracker, _, _, _, _) = FakeIssueTracker::new(make_issue(42, vec![]));
    let runner = SequencedRunner::new(vec![
        agent_success(""),          // implement
        agent_success("<reviewResult>FINDINGS</reviewResult>"),  // review — has findings, loop back
        agent_success(""),          // implement again
        agent_success("<reviewResult>CLEAN</reviewResult>"),     // review — clean
        agent_success(""),          // generate_test_instructions
    ]);
    let ctx = make_context_sequenced(tracker, runner, 10);

    complete_ticket(42, &ctx).unwrap();

    assert_eq!(ctx.iterations_used(), 5);
}

#[test]
fn complete_ticket_marks_hitl_when_budget_exhausted() {
    let (tracker, _, _, skipped, _) = FakeIssueTracker::new(make_issue(42, vec![]));
    let runner = SequencedRunner::new(vec![
        agent_success(""),  // implement uses the only iteration
        // review will hit budget
    ]);
    let ctx = make_context_sequenced(tracker, runner, 1);

    complete_ticket(42, &ctx).unwrap();

    assert!(skipped.borrow().contains(&42));
}

// --- complete_series tests ---

#[test]
fn complete_series_executes_tickets_in_plan_order_sequence() {
    let (tracker, claimed, _, _, _) = FakeIssueTracker::with_issues(vec![
        make_issue(1, vec![]),
        make_issue(2, vec![]),
    ]);
    let runner = SequencedRunner::new(vec![
        agent_success(r#"[{"id": 2}, {"id": 1}]"#),                 // plan_order
        agent_success(""),                                           // implement issue 2
        agent_success("<reviewResult>CLEAN</reviewResult>"),         // review issue 2
        agent_success(""),                                           // instructions issue 2
        agent_success(""),                                           // implement issue 1
        agent_success("<reviewResult>CLEAN</reviewResult>"),         // review issue 1
        agent_success(""),                                           // instructions issue 1
    ]);
    let ctx = make_context_sequenced(tracker, runner, 20);

    complete_series("my-label", &ctx).unwrap();

    assert_eq!(*claimed.borrow(), vec![2, 1]);
}

// --- complete_feature tests ---

#[test]
fn complete_feature_executes_children_reviews_and_generates_instructions_when_clean() {
    let child = make_issue(10, vec![]);
    let (tracker, claimed, _, _, children_calls) = FakeIssueTracker::with_issues(vec![
        make_issue(99, vec!["feature"]),
        child.clone(),
    ]);
    children_calls.borrow_mut().push_back(vec![child]); // first get_children call

    let runner = SequencedRunner::new(vec![
        agent_success(r#"[{"id": 10}]"#),                                        // plan_order
        agent_success(""),                                                        // implement child 10
        agent_success("<reviewResult>CLEAN</reviewResult>"),                     // review child 10
        agent_success(""),                                                        // instructions child 10
        agent_success("<featureReviewResult>CLEAN</featureReviewResult>"),       // feature_review
        agent_success(""),                                                        // feature instructions
    ]);
    let ctx = make_context_sequenced(tracker, runner, 20);

    complete_feature(99, &ctx).unwrap();

    assert!(claimed.borrow().contains(&10));
}

#[test]
fn complete_feature_executes_new_children_after_in_scope_findings() {
    let child_1 = make_issue(10, vec![]);
    let child_2 = make_issue(11, vec![]);
    let (tracker, claimed, _, _, children_calls) = FakeIssueTracker::with_issues(vec![
        make_issue(99, vec!["feature"]),
        child_1.clone(),
        child_2.clone(),
    ]);
    children_calls.borrow_mut().push_back(vec![child_1.clone()]);                  // first get_children: original
    children_calls.borrow_mut().push_back(vec![child_1.clone(), child_2]);        // second: original + review-created

    let runner = SequencedRunner::new(vec![
        agent_success(r#"[{"id": 10}]"#),                                        // plan_order (initial)
        agent_success(""),                                                        // implement child 10
        agent_success("<reviewResult>CLEAN</reviewResult>"),                     // review child 10
        agent_success(""),                                                        // instructions child 10
        agent_success("<featureReviewResult>IN_SCOPE_FINDINGS</featureReviewResult>"), // feature_review — findings
        agent_success(r#"[{"id": 11}]"#),                                        // plan_order (new children)
        agent_success(""),                                                        // implement child 11
        agent_success("<reviewResult>CLEAN</reviewResult>"),                     // review child 11
        agent_success(""),                                                        // instructions child 11
        agent_success("<featureReviewResult>CLEAN</featureReviewResult>"),       // second feature_review
        agent_success(""),                                                        // feature instructions
    ]);
    let ctx = make_context_sequenced(tracker, runner, 30);

    complete_feature(99, &ctx).unwrap();

    assert!(claimed.borrow().contains(&10));
    assert!(claimed.borrow().contains(&11));
}

#[test]
fn complete_feature_marks_hitl_when_second_feature_review_still_has_findings() {
    let child = make_issue(10, vec![]);
    let child_2 = make_issue(11, vec![]);
    let (tracker, _, _, skipped, children_calls) = FakeIssueTracker::with_issues(vec![
        make_issue(99, vec!["feature"]),
        child.clone(),
        child_2.clone(),
    ]);
    children_calls.borrow_mut().push_back(vec![child.clone()]);
    children_calls.borrow_mut().push_back(vec![child.clone(), child_2]);

    let runner = SequencedRunner::new(vec![
        agent_success(r#"[{"id": 10}]"#),                                        // plan_order
        agent_success(""),                                                        // implement 10
        agent_success("<reviewResult>CLEAN</reviewResult>"),                     // review 10
        agent_success(""),                                                        // instructions 10
        agent_success("<featureReviewResult>IN_SCOPE_FINDINGS</featureReviewResult>"), // feature_review 1
        agent_success(r#"[{"id": 11}]"#),                                        // plan_order new
        agent_success(""),                                                        // implement 11
        agent_success("<reviewResult>CLEAN</reviewResult>"),                     // review 11
        agent_success(""),                                                        // instructions 11
        agent_success("<featureReviewResult>IN_SCOPE_FINDINGS</featureReviewResult>"), // feature_review 2 — still findings
    ]);
    let ctx = make_context_sequenced(tracker, runner, 30);

    complete_feature(99, &ctx).unwrap();

    assert!(skipped.borrow().contains(&99));
}
