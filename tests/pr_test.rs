mod common;

use intern::behaviors::{complete_feature, complete_ticket};
use intern::context::Context;
use intern::traits::MergeStrategy;
use std::rc::Rc;
use std::cell::RefCell;

fn make_context_with_remote(
    tracker: common::FakeIssueTracker,
    runner: impl intern::traits::AgentRunner + 'static,
    sc: common::RecordingSourceControl,
    remote: common::RecordingRemoteClient,
    strategy: MergeStrategy,
) -> (Context, tempfile::TempDir) {
    let dir = common::make_prompts_dir();
    let config = common::run_config_with_strategy(&dir, strategy, "main");
    let ctx = Context::new(
        Box::new(tracker),
        Box::new(sc),
        Box::new(remote),
        Box::new(runner),
        Box::new(common::FakeEventSink),
        config,
    );
    (ctx, dir)
}

// --- PerTicket ---

#[test]
fn per_ticket_creates_pr_for_ticket() {
    let (tracker, _, _, _, _) = common::FakeIssueTracker::new(common::make_issue(1, vec![]));
    let (runner, _) = common::FakeRunner::succeeds();
    let (sc, _) = common::recording_source_control("feature/ticket-1");
    let (remote, pr_calls) = common::RecordingRemoteClient::new();
    let (ctx, _dir) =
        make_context_with_remote(tracker, runner, sc, remote, MergeStrategy::PerTicket);

    complete_ticket(1, &ctx, "main").unwrap();

    assert_eq!(pr_calls.borrow().len(), 1);
    assert_eq!(pr_calls.borrow()[0], "feature/ticket-1");
}

// --- Direct ---

#[test]
fn direct_creates_no_pr() {
    let (tracker, _, _, _, _) = common::FakeIssueTracker::new(common::make_issue(1, vec![]));
    let (runner, _) = common::FakeRunner::succeeds();
    let (sc, _) = common::recording_source_control("main");
    let (remote, pr_calls) = common::RecordingRemoteClient::new();
    let (ctx, _dir) =
        make_context_with_remote(tracker, runner, sc, remote, MergeStrategy::Direct);

    complete_ticket(1, &ctx, "main").unwrap();

    assert!(pr_calls.borrow().is_empty());
}

// --- FeatureBranch ---

#[test]
fn feature_branch_creates_pr_for_each_child_and_feature() {
    let child = common::make_issue(10, vec![]);
    let (tracker, _, _, _, children_calls) = common::FakeIssueTracker::with_issues(vec![
        common::make_issue(99, vec!["feature"]),
        child.clone(),
    ]);
    children_calls.borrow_mut().push_back(vec![child]);

    let runner = common::SequencedRunner::new(vec![
        common::agent_success(r#"[{"id": 10}]"#),                                       // plan_order
        common::agent_success(""),                                                       // implement child 10
        common::agent_success(""),                                                       // review child 10 (clean)
        common::agent_success(""),                                                       // instructions child 10
        common::agent_success("<featureReviewResult>CLEAN</featureReviewResult>"),       // feature review
        common::agent_success(""),                                                       // feature instructions
    ]);
    // current_branch returns "feature/ticket-10" so child branch validation passes
    let (sc, _) = common::recording_source_control("feature/ticket-10");
    let (remote, pr_calls) = common::RecordingRemoteClient::new();

    let dir = common::make_prompts_dir();
    let config = common::run_config_with_strategy(&dir, MergeStrategy::FeatureBranch, "main");
    let ctx = Context::new(
        Box::new(tracker),
        Box::new(sc),
        Box::new(remote),
        Box::new(runner),
        Box::new(common::FakeEventSink),
        config,
    );

    complete_feature(99, &ctx, "main").unwrap();

    let calls = pr_calls.borrow();
    assert_eq!(calls.len(), 2, "expected 2 PRs: one per child, one for feature");
    assert!(calls.contains(&"feature/ticket-10".to_string()), "child PR missing");
    assert!(calls.contains(&"feature/ticket-99".to_string()), "feature PR missing");
}
