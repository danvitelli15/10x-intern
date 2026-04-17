mod common;

use intern::behaviors::complete_feature;
use intern::workflows::clear_workflow;

// --- clear_workflow tests ---

#[test]
fn clear_workflow_executes_tickets_in_plan_order_sequence() {
    let (tracker, claimed, _, _, _) = common::FakeIssueTracker::with_issues(vec![
        common::make_issue(1, vec![]),
        common::make_issue(2, vec![]),
    ]);
    let runner = common::SequencedRunner::new(vec![
        common::agent_success(r#"[{"id": 2}, {"id": 1}]"#),          // plan_order
        common::agent_success(""),                                     // implement issue 2
        common::agent_success("<reviewResult>CLEAN</reviewResult>"),   // review issue 2
        common::agent_success(""),                                     // instructions issue 2
        common::agent_success(""),                                     // implement issue 1
        common::agent_success("<reviewResult>CLEAN</reviewResult>"),   // review issue 1
        common::agent_success(""),                                     // instructions issue 1
    ]);
    let (ctx, _dir) = common::make_context_sequenced(tracker, runner, 20);

    clear_workflow("my-label", &ctx).unwrap();

    assert_eq!(*claimed.borrow(), vec![2, 1]);
}

// --- complete_feature behavior tests ---

#[test]
fn complete_feature_executes_children_reviews_and_generates_instructions_when_clean() {
    let child = common::make_issue(10, vec![]);
    let (tracker, claimed, _, _, children_calls) = common::FakeIssueTracker::with_issues(vec![
        common::make_issue(99, vec!["feature"]),
        child.clone(),
    ]);
    children_calls.borrow_mut().push_back(vec![child]);

    let runner = common::SequencedRunner::new(vec![
        common::agent_success(r#"[{"id": 10}]"#),                                   // plan_order
        common::agent_success(""),                                                   // implement child 10
        common::agent_success("<reviewResult>CLEAN</reviewResult>"),                 // review child 10
        common::agent_success(""),                                                   // instructions child 10
        common::agent_success("<featureReviewResult>CLEAN</featureReviewResult>"),   // feature_review
        common::agent_success(""),                                                   // feature instructions
    ]);
    let (ctx, _dir) = common::make_context_sequenced(tracker, runner, 20);

    complete_feature(99, &ctx, "main").unwrap();

    assert!(claimed.borrow().contains(&10));
}

#[test]
fn complete_feature_executes_new_children_after_in_scope_findings() {
    let child_1 = common::make_issue(10, vec![]);
    let child_2 = common::make_issue(11, vec![]);
    let (tracker, claimed, _, _, children_calls) = common::FakeIssueTracker::with_issues(vec![
        common::make_issue(99, vec!["feature"]),
        child_1.clone(),
        child_2.clone(),
    ]);
    children_calls.borrow_mut().push_back(vec![child_1.clone()]);
    children_calls.borrow_mut().push_back(vec![child_1.clone(), child_2]);

    let runner = common::SequencedRunner::new(vec![
        common::agent_success(r#"[{"id": 10}]"#),                                             // plan_order (initial)
        common::agent_success(""),                                                             // implement child 10
        common::agent_success("<reviewResult>CLEAN</reviewResult>"),                           // review child 10
        common::agent_success(""),                                                             // instructions child 10
        common::agent_success("<featureReviewResult>IN_SCOPE_FINDINGS</featureReviewResult>"), // feature_review — findings
        common::agent_success(r#"[{"id": 11}]"#),                                             // plan_order (new children)
        common::agent_success(""),                                                             // implement child 11
        common::agent_success("<reviewResult>CLEAN</reviewResult>"),                           // review child 11
        common::agent_success(""),                                                             // instructions child 11
        common::agent_success("<featureReviewResult>CLEAN</featureReviewResult>"),             // second feature_review
        common::agent_success(""),                                                             // feature instructions
    ]);
    let (ctx, _dir) = common::make_context_sequenced(tracker, runner, 30);

    complete_feature(99, &ctx, "main").unwrap();

    assert!(claimed.borrow().contains(&10));
    assert!(claimed.borrow().contains(&11));
}

#[test]
fn complete_feature_marks_skipped_when_second_feature_review_still_has_findings() {
    let child = common::make_issue(10, vec![]);
    let child_2 = common::make_issue(11, vec![]);
    let (tracker, _, _, skipped, children_calls) = common::FakeIssueTracker::with_issues(vec![
        common::make_issue(99, vec!["feature"]),
        child.clone(),
        child_2.clone(),
    ]);
    children_calls.borrow_mut().push_back(vec![child.clone()]);
    children_calls.borrow_mut().push_back(vec![child.clone(), child_2]);

    let runner = common::SequencedRunner::new(vec![
        common::agent_success(r#"[{"id": 10}]"#),                                             // plan_order
        common::agent_success(""),                                                             // implement 10
        common::agent_success("<reviewResult>CLEAN</reviewResult>"),                           // review 10
        common::agent_success(""),                                                             // instructions 10
        common::agent_success("<featureReviewResult>IN_SCOPE_FINDINGS</featureReviewResult>"), // feature_review 1
        common::agent_success(r#"[{"id": 11}]"#),                                             // plan_order new
        common::agent_success(""),                                                             // implement 11
        common::agent_success("<reviewResult>CLEAN</reviewResult>"),                           // review 11
        common::agent_success(""),                                                             // instructions 11
        common::agent_success("<featureReviewResult>IN_SCOPE_FINDINGS</featureReviewResult>"), // feature_review 2 — still findings
    ]);
    let (ctx, _dir) = common::make_context_sequenced(tracker, runner, 30);

    complete_feature(99, &ctx, "main").unwrap();

    assert!(skipped.borrow().contains(&99));
}
