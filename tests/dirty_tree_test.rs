mod common;

use intern::behaviors::complete_ticket;
use intern::context::Context;
use intern::traits::{DirtyBehavior, MergeStrategy, RunConfig};

fn make_context_dirty(
    tracker: common::FakeIssueTracker,
    runner: common::SequencedRunner,
    sc: common::RecordingSourceControl,
    on_dirty_after_commit: DirtyBehavior,
    on_dirty_no_commits: DirtyBehavior,
) -> (Context, tempfile::TempDir) {
    let dir = common::make_prompts_dir();
    let config = RunConfig {
        max_iterations: 10,
        merge_strategy: MergeStrategy::Direct,
        base_branch: "main".to_string(),
        use_worktree: false,
        on_dirty_after_commit,
        on_dirty_no_commits,
        dry_run: false,
        repo_context: String::new(),
        work_directory: dir.path().to_path_buf(),
    };
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

fn sequenced_clean_run() -> common::SequencedRunner {
    common::SequencedRunner::new(vec![
        common::agent_success(""),
        common::agent_success("<reviewResult>CLEAN</reviewResult>"),
        common::agent_success(""),
    ])
}

// --- on_dirty_no_commits ---

#[test]
fn no_commits_fail_returns_error() {
    let (tracker, _, _, _, _) = common::FakeIssueTracker::new(common::make_issue(1, vec![]));
    let (sc, _, _) = common::recording_source_control_full(
        "main",
        true,  // dirty
        false, // no commits
    );
    let (ctx, _dir) = make_context_dirty(
        tracker,
        sequenced_clean_run(),
        sc,
        DirtyBehavior::Warn,
        DirtyBehavior::Fail,
    );

    let result = complete_ticket(1, &ctx, "main");

    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("no commits"), "expected 'no commits' in: {msg}");
}

#[test]
fn no_commits_warn_continues() {
    let (tracker, _, _, _, _) = common::FakeIssueTracker::new(common::make_issue(1, vec![]));
    let (sc, _, _) = common::recording_source_control_full(
        "main",
        false, // clean
        false, // no commits
    );
    let (ctx, _dir) = make_context_dirty(
        tracker,
        sequenced_clean_run(),
        sc,
        DirtyBehavior::Warn,
        DirtyBehavior::Warn,
    );

    let result = complete_ticket(1, &ctx, "main");

    assert!(result.is_ok());
}

#[test]
fn no_commits_commit_stages_and_commits_all() {
    let (tracker, _, _, _, _) = common::FakeIssueTracker::new(common::make_issue(1, vec![]));
    let (sc, commits, stages) = common::recording_source_control_full(
        "main",
        false, // clean
        false, // no commits
    );
    let (ctx, _dir) = make_context_dirty(
        tracker,
        sequenced_clean_run(),
        sc,
        DirtyBehavior::Warn,
        DirtyBehavior::Commit,
    );

    let result = complete_ticket(1, &ctx, "main");

    assert!(result.is_ok());
    assert_eq!(stages.borrow().len(), 1, "expected one stage call");
    assert!(stages.borrow()[0].is_none(), "expected stage-all (None paths)");
    assert_eq!(commits.borrow().len(), 1, "expected one commit");
    assert!(commits.borrow()[0].contains("issue #1"), "expected issue id in commit message");
}

// --- on_dirty_after_commit ---

#[test]
fn dirty_after_commit_fail_returns_error() {
    let (tracker, _, _, _, _) = common::FakeIssueTracker::new(common::make_issue(1, vec![]));
    let (sc, _, _) = common::recording_source_control_full(
        "main",
        true, // dirty
        true, // has commits
    );
    let (ctx, _dir) = make_context_dirty(
        tracker,
        sequenced_clean_run(),
        sc,
        DirtyBehavior::Fail,
        DirtyBehavior::Warn,
    );

    let result = complete_ticket(1, &ctx, "main");

    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("uncommitted"), "expected 'uncommitted' in: {msg}");
}

#[test]
fn dirty_after_commit_warn_continues() {
    let (tracker, _, _, _, _) = common::FakeIssueTracker::new(common::make_issue(1, vec![]));
    let (sc, _, _) = common::recording_source_control_full(
        "main",
        true, // dirty
        true, // has commits
    );
    let (ctx, _dir) = make_context_dirty(
        tracker,
        sequenced_clean_run(),
        sc,
        DirtyBehavior::Warn,
        DirtyBehavior::Warn,
    );

    let result = complete_ticket(1, &ctx, "main");

    assert!(result.is_ok());
}

#[test]
fn dirty_after_commit_commit_stages_and_commits_remaining() {
    let (tracker, _, _, _, _) = common::FakeIssueTracker::new(common::make_issue(1, vec![]));
    let (sc, commits, stages) = common::recording_source_control_full(
        "main",
        true, // dirty
        true, // has commits
    );
    let (ctx, _dir) = make_context_dirty(
        tracker,
        sequenced_clean_run(),
        sc,
        DirtyBehavior::Commit,
        DirtyBehavior::Warn,
    );

    let result = complete_ticket(1, &ctx, "main");

    assert!(result.is_ok());
    assert_eq!(stages.borrow().len(), 1);
    assert!(stages.borrow()[0].is_none());
    assert_eq!(commits.borrow().len(), 1);
    assert!(commits.borrow()[0].contains("issue #1"));
}
