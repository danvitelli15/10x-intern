use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;

use crate::traits::CommandRunner;

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
