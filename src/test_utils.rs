use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;

use crate::traits::CommandRunner;

/// A test double for CommandRunner.
/// Returns a canned response and records every call for assertion.
///
/// Usage:
///   let (adapter, calls) = adapter("");          // don't care what was called
///   let (adapter, calls) = adapter("some json"); // care about parsing
///   calls.borrow()[0]                            // inspect what was called
pub struct FakeCommandRunner {
    pub calls: Rc<RefCell<Vec<Vec<String>>>>,
    pub response: String,
}

impl CommandRunner for FakeCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        let mut call = vec![program.to_string()];
        call.extend(args.iter().map(|s| s.to_string()));
        self.calls.borrow_mut().push(call);
        Ok(self.response.clone())
    }
}

/// Construct a FakeCommandRunner and return it alongside the shared calls handle.
pub fn fake_runner(response: &str) -> (FakeCommandRunner, Rc<RefCell<Vec<Vec<String>>>>) {
    let calls = Rc::new(RefCell::new(vec![]));
    let runner = FakeCommandRunner { calls: calls.clone(), response: response.to_string() };
    (runner, calls)
}
