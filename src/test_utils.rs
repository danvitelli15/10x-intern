use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;

use crate::traits::{
    CommandRunner, Event, EventSink, Issue, IssueTracker, IssueType, RemoteClient, SourceControl,
};

/// A test double for CommandRunner.
/// Returns a canned response and records every call for assertion.
///
/// Usage:
///   let (runner, calls) = fake_runner("");           // don't care what was called
///   let (runner, calls) = fake_runner("some json");  // care about parsing
///   let (runner, calls) = fake_failing_runner();     // simulate command failure
///   calls.borrow()[0]                                // inspect what was called
pub struct FakeCommandRunner {
    pub calls: Rc<RefCell<Vec<Vec<String>>>>,
    pub response: String,
    pub fail: bool,
}

impl CommandRunner for FakeCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        let mut call = vec![program.to_string()];
        call.extend(args.iter().map(|s| s.to_string()));
        self.calls.borrow_mut().push(call);
        if self.fail {
            anyhow::bail!("command failed")
        } else {
            Ok(self.response.clone())
        }
    }
}

/// Construct a FakeCommandRunner that succeeds with the given response.
pub fn fake_runner(response: &str) -> (FakeCommandRunner, Rc<RefCell<Vec<Vec<String>>>>) {
    let calls = Rc::new(RefCell::new(vec![]));
    let runner = FakeCommandRunner { calls: calls.clone(), response: response.to_string(), fail: false };
    (runner, calls)
}

/// Construct a FakeCommandRunner that always returns an error.
pub fn fake_failing_runner() -> (FakeCommandRunner, Rc<RefCell<Vec<Vec<String>>>>) {
    let calls = Rc::new(RefCell::new(vec![]));
    let runner = FakeCommandRunner { calls: calls.clone(), response: String::new(), fail: true };
    (runner, calls)
}

pub struct StubIssueTracker;
impl IssueTracker for StubIssueTracker {
    fn get_issue(&self, id: u64) -> Result<Issue> {
        Ok(Issue { id, title: "T".into(), body: "B".into(), labels: vec![] })
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

pub struct StubSourceControl;
impl SourceControl for StubSourceControl {
    fn create_branch(&self, _: &str) -> Result<()> { Ok(()) }
    fn current_branch(&self) -> Result<String> { Ok("main".into()) }
    fn diff_from_base(&self, _: &str) -> Result<String> { Ok("".into()) }
    fn stage(&self, _: Option<&[&str]>) -> Result<()> { Ok(()) }
    fn commit(&self, _: &str) -> Result<()> { Ok(()) }
}

pub struct StubRemoteClient;
impl RemoteClient for StubRemoteClient {
    fn create_pr(&self, _: &str, _: &str, _: &str) -> Result<String> { Ok("".into()) }
}

pub struct StubEventSink;
impl EventSink for StubEventSink {
    fn emit(&self, _: Event) {}
}
