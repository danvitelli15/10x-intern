mod client;
pub mod types;

use anyhow::Result;

use crate::github::types::GhIssue;
use crate::traits::{CommandRunner, Issue, IssueTracker, RemoteClient};

fn parse_id_from_url(url: &str) -> Result<u64> {
    url.trim()
        .split('/')
        .last()
        .ok_or_else(|| anyhow::anyhow!("unexpected URL format: {}", url))?
        .parse::<u64>()
        .map_err(|e| anyhow::anyhow!("failed to parse ID from URL '{}': {}", url.trim(), e))
}

fn into_issue(gh: GhIssue) -> Issue {
    Issue {
        id: gh.number,
        title: gh.title,
        body: gh.body,
        labels: gh.labels.into_iter().map(|l| l.name).collect(),
    }
}

/// Adapter implementing both IssueTracker and RemoteClient via the `gh` CLI.
pub struct GithubAdapter {
    repo: String,
    runner: Box<dyn CommandRunner>,
}

impl GithubAdapter {
    pub fn new(repo: &str, runner: Box<dyn CommandRunner>) -> Self {
        Self {
            repo: repo.to_string(),
            runner,
        }
    }
}

impl IssueTracker for GithubAdapter {
    fn get_issue(&self, id: u64) -> Result<Issue> {
        let output = self.runner.run(
            "gh",
            &[
                "issue",
                "view",
                &id.to_string(),
                "--repo",
                &self.repo,
                "--json",
                "number,title,body,labels",
            ],
        )?;
        let gh_issue: GhIssue = serde_json::from_str(&output)?;
        Ok(into_issue(gh_issue))
    }

    fn get_children(&self, id: u64) -> Result<Vec<Issue>> {
        let path = format!("/repos/{}/issues/{}/sub_issues", self.repo, id);
        let output = self.runner.run("gh", &["api", &path])?;
        let gh_issues: Vec<GhIssue> = serde_json::from_str(&output)?;
        Ok(gh_issues.into_iter().map(into_issue).collect())
    }

    fn get_issues_by_label(&self, label: &str) -> Result<Vec<Issue>> {
        let output = self.runner.run(
            "gh",
            &[
                "issue",
                "list",
                "--repo",
                &self.repo,
                "--label",
                label,
                "--json",
                "number,title,body,labels",
            ],
        )?;
        let gh_issues: Vec<GhIssue> = serde_json::from_str(&output)?;
        Ok(gh_issues.into_iter().map(into_issue).collect())
    }

    fn claim_issue(&self, id: u64) -> Result<()> {
        self.runner.run(
            "gh",
            &[
                "issue",
                "comment",
                &id.to_string(),
                "--repo",
                &self.repo,
                "--body",
                "Claiming this issue.",
            ],
        )?;
        Ok(())
    }

    fn complete_issue(&self, id: u64) -> Result<()> {
        self.runner.run(
            "gh",
            &[
                "issue",
                "edit",
                &id.to_string(),
                "--repo",
                &self.repo,
                "--add-label",
                "agent-complete",
            ],
        )?;
        Ok(())
    }

    fn skip_issue(&self, id: u64) -> Result<()> {
        self.runner.run(
            "gh",
            &[
                "issue",
                "edit",
                &id.to_string(),
                "--repo",
                &self.repo,
                "--add-label",
                "hitl",
            ],
        )?;
        Ok(())
    }

    fn post_comment(&self, id: u64, body: &str) -> Result<()> {
        self.runner.run(
            "gh",
            &[
                "issue",
                "comment",
                &id.to_string(),
                "--repo",
                &self.repo,
                "--body",
                body,
            ],
        )?;
        Ok(())
    }

    fn create_child_issue(&self, parent_id: u64, title: &str, body: &str) -> Result<Issue> {
        let url = self.runner.run(
            "gh",
            &[
                "issue", "create", "--repo", &self.repo, "--title", title, "--body", body,
            ],
        )?;
        let child_id = parse_id_from_url(&url)?;

        let path = format!("/repos/{}/issues/{}/sub_issues", self.repo, parent_id);
        let link_field = format!("sub_issue_id={}", child_id);
        self.runner
            .run("gh", &["api", &path, "--method", "POST", "-f", &link_field])?;

        Ok(Issue {
            id: child_id,
            title: title.to_string(),
            body: body.to_string(),
            labels: vec![],
        })
    }
}

impl RemoteClient for GithubAdapter {
    fn create_pr(&self, title: &str, body: &str, branch: &str) -> Result<String> {
        let url = self.runner.run(
            "gh",
            &[
                "pr", "create", "--repo", &self.repo, "--title", title, "--body", body, "--head",
                branch,
            ],
        )?;
        Ok(url.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    // Returns canned output — used when we only care about parsing, not what was called.
    struct FakeCommandRunner {
        response: String,
    }

    impl FakeCommandRunner {
        fn new(response: &str) -> Self {
            Self {
                response: response.to_string(),
            }
        }
    }

    impl CommandRunner for FakeCommandRunner {
        fn run(&self, _program: &str, _args: &[&str]) -> Result<String> {
            Ok(self.response.clone())
        }
    }

    fn adapter(response: &str) -> GithubAdapter {
        GithubAdapter::new("owner/repo", Box::new(FakeCommandRunner::new(response)))
    }

    // Records every command that was run — used when we care about what args were passed.
    // Rc<RefCell<...>>: Rc lets the test and the runner share ownership of the vec;
    // RefCell lets us mutate it through a shared reference (needed because CommandRunner::run takes &self).
    struct RecordingCommandRunner {
        calls: Rc<RefCell<Vec<Vec<String>>>>,
        response: String,
    }

    impl CommandRunner for RecordingCommandRunner {
        fn run(&self, program: &str, args: &[&str]) -> Result<String> {
            let mut call = vec![program.to_string()];
            call.extend(args.iter().map(|s| s.to_string()));
            self.calls.borrow_mut().push(call);
            Ok(self.response.clone())
        }
    }

    fn recording_adapter(response: &str) -> (GithubAdapter, Rc<RefCell<Vec<Vec<String>>>>) {
        let calls = Rc::new(RefCell::new(vec![]));
        let runner = RecordingCommandRunner {
            calls: calls.clone(),
            response: response.to_string(),
        };
        let adapter = GithubAdapter::new("owner/repo", Box::new(runner));
        (adapter, calls)
    }

    #[test]
    fn claim_issue_posts_comment() {
        let (adapter, calls) = recording_adapter("");

        adapter.claim_issue(42).unwrap();

        let calls = calls.borrow();
        assert_eq!(calls.len(), 1);
        // should be: gh issue comment 42 --repo owner/repo --body "..."
        assert_eq!(calls[0][0], "gh");
        assert_eq!(calls[0][1], "issue");
        assert_eq!(calls[0][2], "comment");
        assert_eq!(calls[0][3], "42");
        assert!(calls[0].contains(&"--repo".to_string()));
        assert!(calls[0].contains(&"owner/repo".to_string()));
        assert!(calls[0].contains(&"--body".to_string()));
    }

    #[test]
    fn get_issues_by_label_parses_list() {
        let json = r#"[
            {"number": 1, "title": "First", "body": "body one", "labels": [{"name": "aft"}]},
            {"number": 2, "title": "Second", "body": "body two", "labels": [{"name": "aft"}, {"name": "bug"}]}
        ]"#;

        let issues = adapter(json).get_issues_by_label("aft").unwrap();

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].id, 1);
        assert_eq!(issues[1].labels, vec!["aft", "bug"]);
    }

    #[test]
    fn complete_issue_adds_label() {
        let (adapter, calls) = recording_adapter("");

        adapter.complete_issue(7).unwrap();

        let calls = calls.borrow();
        assert_eq!(calls[0][0], "gh");
        assert_eq!(calls[0][1], "issue");
        assert_eq!(calls[0][2], "edit");
        assert_eq!(calls[0][3], "7");
        assert!(calls[0].contains(&"--add-label".to_string()));
        assert!(calls[0].contains(&"agent-complete".to_string()));
    }

    #[test]
    fn skip_issue_adds_hitl_label() {
        let (adapter, calls) = recording_adapter("");

        adapter.skip_issue(9).unwrap();

        let calls = calls.borrow();
        assert_eq!(calls[0][2], "edit");
        assert_eq!(calls[0][3], "9");
        assert!(calls[0].contains(&"--add-label".to_string()));
        assert!(calls[0].contains(&"hitl".to_string()));
    }

    #[test]
    fn post_comment_sends_body() {
        let (adapter, calls) = recording_adapter("");

        adapter.post_comment(5, "hello from the agent").unwrap();

        let calls = calls.borrow();
        assert_eq!(calls[0][2], "comment");
        assert_eq!(calls[0][3], "5");
        assert!(calls[0].contains(&"--body".to_string()));
        assert!(calls[0].contains(&"hello from the agent".to_string()));
    }

    #[test]
    fn create_child_issue_creates_and_links() {
        let url = "https://github.com/owner/repo/issues/99\n";
        let (adapter, calls) = recording_adapter(url);

        let issue = adapter
            .create_child_issue(1, "Child title", "Child body")
            .unwrap();

        let calls = calls.borrow();
        // first call: gh issue create
        assert!(calls[0].contains(&"create".to_string()));
        assert!(calls[0].contains(&"Child title".to_string()));
        assert!(calls[0].contains(&"Child body".to_string()));
        // second call: gh api to link as sub-issue under parent 1
        assert!(calls[1].contains(&"api".to_string()));
        assert!(calls[1].contains(&"/repos/owner/repo/issues/1/sub_issues".to_string()));
        assert!(calls[1].contains(&"sub_issue_id=99".to_string()));
        drop(calls);

        assert_eq!(issue.id, 99);
        assert_eq!(issue.title, "Child title");
        assert_eq!(issue.body, "Child body");
    }

    #[test]
    fn get_children_parses_sub_issues() {
        let json = r#"[
            {"number": 10, "title": "Sub one", "body": "body", "labels": []},
            {"number": 11, "title": "Sub two", "body": "body", "labels": [{"name": "aft"}]}
        ]"#;

        let issues = adapter(json).get_children(5).unwrap();

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].id, 10);
        assert_eq!(issues[1].labels, vec!["aft"]);
    }

    #[test]
    fn create_pr_returns_url() {
        let url = "https://github.com/owner/repo/pull/5\n";
        let (adapter, calls) = recording_adapter(url);

        let result = adapter
            .create_pr("My PR", "PR body", "feature/123")
            .unwrap();

        let calls = calls.borrow();
        assert!(calls[0].contains(&"create".to_string()));
        assert!(calls[0].contains(&"My PR".to_string()));
        assert!(calls[0].contains(&"feature/123".to_string()));
        drop(calls);

        assert_eq!(result, "https://github.com/owner/repo/pull/5");
    }

    #[test]
    fn get_issue_parses_response() {
        let json = r#"{
            "number": 42,
            "title": "Add user authentication",
            "body": "We need auth",
            "labels": [{"name": "feature"}, {"name": "aft"}]
        }"#;

        let issue = adapter(json).get_issue(42).unwrap();

        assert_eq!(issue.id, 42);
        assert_eq!(issue.title, "Add user authentication");
        assert_eq!(issue.body, "We need auth");
        assert_eq!(issue.labels, vec!["feature", "aft"]);
    }
}
