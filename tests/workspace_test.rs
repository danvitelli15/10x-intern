mod common;

use intern::behaviors::{complete_ticket, complete_feature};
use intern::context::Context;
use intern::traits::MergeStrategy;

fn make_context_with_sc(
    tracker: common::FakeIssueTracker,
    runner: common::FakeRunner,
    sc: common::RecordingSourceControl,
    strategy: MergeStrategy,
    base_branch: &str,
) -> (Context, tempfile::TempDir) {
    let dir = common::make_prompts_dir();
    let config = common::run_config_with_strategy(&dir, strategy, base_branch);
    let ctx = Context::new(
        Box::new(tracker),
        Box::new(sc),
        Box::new(common::FakeRemoteClient),
        Box::new(runner),
        Box::new(common::FakeEventSink),
        config,
    );
    (ctx, dir)
}

// --- Branch creation ---

#[test]
fn complete_ticket_per_ticket_creates_branch_from_base() {
    let (tracker, _, _, _, _) = common::FakeIssueTracker::new(common::make_issue(42, vec![]));
    let (runner, _) = common::FakeRunner::succeeds();
    let (sc, branches) = common::recording_source_control("feature/ticket-42");
    let (ctx, _dir) = make_context_with_sc(tracker, runner, sc, MergeStrategy::PerTicket, "main");

    complete_ticket(42, &ctx, "main").unwrap();

    let branches = branches.borrow();
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0], ("feature/ticket-42".to_string(), "main".to_string()));
}

#[test]
fn complete_ticket_fails_hard_when_branch_changes_after_implement() {
    let (tracker, _, _, _, _) = common::FakeIssueTracker::new(common::make_issue(42, vec![]));
    let (runner, _) = common::FakeRunner::succeeds();
    // current_branch() returns "main" — not the branch we created
    let (sc, _) = common::recording_source_control("main");
    let (ctx, _dir) = make_context_with_sc(tracker, runner, sc, MergeStrategy::PerTicket, "main");

    let result = complete_ticket(42, &ctx, "main");

    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("branch mismatch"), "expected 'branch mismatch' in: {msg}");
    assert!(msg.contains("feature/ticket-42"), "expected branch name in: {msg}");
}

#[test]
fn complete_ticket_direct_creates_no_branch() {
    let (tracker, _, _, _, _) = common::FakeIssueTracker::new(common::make_issue(42, vec![]));
    let (runner, _) = common::FakeRunner::succeeds();
    // current_branch() returns "main" — would fail validation if validation ran
    let (sc, branches) = common::recording_source_control("main");
    let (ctx, _dir) = make_context_with_sc(tracker, runner, sc, MergeStrategy::Direct, "main");

    complete_ticket(42, &ctx, "main").unwrap(); // must not fail

    assert!(branches.borrow().is_empty(), "Direct should create no branches");
}

// --- Feature branch strategy ---

#[test]
fn complete_feature_feature_branch_creates_feature_branch_and_children_branch_from_it() {
    let child = common::make_issue(10, vec![]);
    let (tracker, _, _, _, children_calls) = common::FakeIssueTracker::with_issues(vec![
        common::make_issue(99, vec!["feature"]),
        child.clone(),
    ]);
    children_calls.borrow_mut().push_back(vec![child]);

    let runner = common::SequencedRunner::new(vec![
        common::agent_success(r#"[{"id": 10}]"#),                      // plan_order
        common::agent_success(""),                                      // implement child 10
        common::agent_success("<reviewResult>CLEAN</reviewResult>"),    // review child 10
        common::agent_success(""),                                      // instructions child 10
        common::agent_success("<featureReviewResult>CLEAN</featureReviewResult>"), // feature review
        common::agent_success(""),                                      // feature instructions
    ]);

    let dir = common::make_prompts_dir();
    let config = common::run_config_with_strategy(&dir, MergeStrategy::FeatureBranch, "main");
    // current_branch returns "feature/ticket-10" so child validation passes
    let (sc, branches) = common::recording_source_control("feature/ticket-10");
    let ctx = intern::context::Context::new(
        Box::new(tracker),
        Box::new(sc),
        Box::new(common::FakeRemoteClient),
        Box::new(runner),
        Box::new(common::FakeEventSink),
        config,
    );

    complete_feature(99, &ctx, "main").unwrap();

    let branches = branches.borrow();
    // First branch: feature branch from main
    assert_eq!(branches[0], ("feature/ticket-99".to_string(), "main".to_string()));
    // Second branch: child ticket branches from the feature branch
    assert_eq!(branches[1], ("feature/ticket-10".to_string(), "feature/ticket-99".to_string()));
}

#[test]
fn complete_feature_per_ticket_creates_no_feature_branch_children_branch_from_base() {
    let child = common::make_issue(10, vec![]);
    let (tracker, _, _, _, children_calls) = common::FakeIssueTracker::with_issues(vec![
        common::make_issue(99, vec!["feature"]),
        child.clone(),
    ]);
    children_calls.borrow_mut().push_back(vec![child]);

    let runner = common::SequencedRunner::new(vec![
        common::agent_success(r#"[{"id": 10}]"#),
        common::agent_success(""),
        common::agent_success("<reviewResult>CLEAN</reviewResult>"),
        common::agent_success(""),
        common::agent_success("<featureReviewResult>CLEAN</featureReviewResult>"),
        common::agent_success(""),
    ]);

    let dir = common::make_prompts_dir();
    let config = common::run_config_with_strategy(&dir, MergeStrategy::PerTicket, "main");
    let (sc, branches) = common::recording_source_control("feature/ticket-10");
    let ctx = intern::context::Context::new(
        Box::new(tracker),
        Box::new(sc),
        Box::new(common::FakeRemoteClient),
        Box::new(runner),
        Box::new(common::FakeEventSink),
        config,
    );

    complete_feature(99, &ctx, "main").unwrap();

    let branches = branches.borrow();
    // Only child ticket branch — no feature-level branch — child branches from "main"
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0], ("feature/ticket-10".to_string(), "main".to_string()));
}
