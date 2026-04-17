mod common;

use intern::actions::implement;
use intern::behaviors::complete_ticket;

// --- implement action tests ---

#[test]
fn implement_fn_claims_issue_before_running_agent() {
    let (tracker, claimed, _, _, _) = common::FakeIssueTracker::new(common::make_issue(42, vec![]));
    let (runner, _) = common::FakeRunner::succeeds();
    let (ctx, _dir) = common::make_context(tracker, runner);

    implement(42, &ctx).unwrap();

    assert!(claimed.borrow().contains(&42));
}

#[test]
fn implement_fn_runs_agent_with_issue_content_in_prompt() {
    let (tracker, _, _, _, _) = common::FakeIssueTracker::new(common::make_issue(42, vec![]));
    let (runner, prompt) = common::FakeRunner::succeeds();
    let (ctx, _dir) = common::make_context(tracker, runner);

    implement(42, &ctx).unwrap();

    let prompt = prompt.borrow();
    let prompt = prompt.as_ref().unwrap();
    assert!(prompt.contains("Issue 42"));
    assert!(prompt.contains("Body of issue 42"));
}

#[test]
fn implement_fn_marks_complete_when_agent_succeeds() {
    let (tracker, _, completed, _, _) = common::FakeIssueTracker::new(common::make_issue(42, vec![]));
    let (runner, _) = common::FakeRunner::succeeds();
    let (ctx, _dir) = common::make_context(tracker, runner);

    implement(42, &ctx).unwrap();

    assert!(completed.borrow().contains(&42));
}

#[test]
fn implement_fn_skips_hitl_issues_without_running_agent() {
    let (tracker, claimed, _, _, _) = common::FakeIssueTracker::new(common::make_issue(42, vec!["hitl"]));
    let (runner, prompt) = common::FakeRunner::succeeds();
    let (ctx, _dir) = common::make_context(tracker, runner);

    implement(42, &ctx).unwrap();

    assert!(claimed.borrow().is_empty());
    assert!(prompt.borrow().is_none());
}

#[test]
fn implement_fn_does_not_mark_complete_when_agent_fails() {
    let (tracker, _, completed, _, _) = common::FakeIssueTracker::new(common::make_issue(42, vec![]));
    let runner = common::FakeRunner::fails();
    let (ctx, _dir) = common::make_context(tracker, runner);

    implement(42, &ctx).unwrap();

    assert!(completed.borrow().is_empty());
}

// --- complete_ticket behavior tests ---

#[test]
fn complete_ticket_runs_implement_review_and_instructions_when_clean() {
    let (tracker, _, _, _, _) = common::FakeIssueTracker::new(common::make_issue(42, vec![]));
    let runner = common::SequencedRunner::new(vec![
        common::agent_success(""),        // implement
        common::agent_success("CLEAN"),   // review
        common::agent_success(""),        // generate_test_instructions
    ]);
    let (ctx, _dir) = common::make_context_sequenced(tracker, runner, 10);

    complete_ticket(42, &ctx, "main").unwrap();

    assert_eq!(ctx.iterations_used(), 3);
}

#[test]
fn complete_ticket_loops_when_review_has_findings() {
    let (tracker, _, _, _, _) = common::FakeIssueTracker::new(common::make_issue(42, vec![]));
    let runner = common::SequencedRunner::new(vec![
        common::agent_success(""),                                              // implement
        common::agent_success("<reviewResult>FINDINGS</reviewResult>"),         // review — findings
        common::agent_success(""),                                              // implement again
        common::agent_success("<reviewResult>CLEAN</reviewResult>"),            // review — clean
        common::agent_success(""),                                              // generate_test_instructions
    ]);
    let (ctx, _dir) = common::make_context_sequenced(tracker, runner, 10);

    complete_ticket(42, &ctx, "main").unwrap();

    assert_eq!(ctx.iterations_used(), 5);
}

#[test]
fn complete_ticket_marks_skipped_when_budget_exhausted() {
    let (tracker, _, _, skipped, _) = common::FakeIssueTracker::new(common::make_issue(42, vec![]));
    let runner = common::SequencedRunner::new(vec![
        common::agent_success(""), // implement uses the only iteration
        // review will hit budget
    ]);
    let (ctx, _dir) = common::make_context_sequenced(tracker, runner, 1);

    complete_ticket(42, &ctx, "main").unwrap();

    assert!(skipped.borrow().contains(&42));
}
