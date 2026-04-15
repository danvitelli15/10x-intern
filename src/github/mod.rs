pub mod types;

use anyhow::Result;

use crate::github::types::GhIssue;
use crate::traits::{CommandRunner, Issue, IssueTracker, IssueType, RemoteClient};

fn parse_id_from_url(url: &str) -> Result<u64> {
    url.trim()
        .split('/')
        .next_back()
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
        log::trace!("gh: fetching issue #{id} from {}", self.repo);
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
        log::trace!("gh: issue #{id} — '{}'", gh_issue.title);
        Ok(into_issue(gh_issue))
    }

    fn get_children(&self, id: u64) -> Result<Vec<Issue>> {
        log::trace!("gh: fetching sub-issues for #{id}");
        let path = format!("/repos/{}/issues/{}/sub_issues", self.repo, id);
        let output = self.runner.run("gh", &["api", &path])?;
        let gh_issues: Vec<GhIssue> = serde_json::from_str(&output)?;
        log::debug!("gh: issue #{id} has {} sub-issue(s)", gh_issues.len());
        Ok(gh_issues.into_iter().map(into_issue).collect())
    }

    fn get_issues_by_label(&self, label: &str) -> Result<Vec<Issue>> {
        log::debug!("gh: listing issues with label '{label}' in {}", self.repo);
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
        log::debug!("gh: found {} issue(s) with label '{label}'", gh_issues.len());
        Ok(gh_issues.into_iter().map(into_issue).collect())
    }

    fn claim_issue(&self, id: u64) -> Result<()> {
        log::debug!("gh: posting claim comment on issue #{id}");
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
        log::debug!("gh: labeling issue #{id} as agent-complete");
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
        log::debug!("gh: labeling issue #{id} as hitl (skipping)");
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
        log::debug!("gh: posting comment on issue #{id}");
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
        log::info!("gh: creating child issue for #{parent_id}: '{title}'");
        let url = self.runner.run(
            "gh",
            &[
                "issue", "create", "--repo", &self.repo, "--title", title, "--body", body,
            ],
        )?;
        let child_id = parse_id_from_url(&url)?;
        log::debug!("gh: child issue #{child_id} created, linking to parent #{parent_id}");

        let path = format!("/repos/{}/issues/{}/sub_issues", self.repo, parent_id);
        let link_field = format!("sub_issue_id={}", child_id);
        self.runner
            .run("gh", &["api", &path, "--method", "POST", "-f", &link_field])?;
        log::debug!("gh: #{child_id} linked as sub-issue of #{parent_id}");

        Ok(Issue {
            id: child_id,
            title: title.to_string(),
            body: body.to_string(),
            labels: vec![],
        })
    }

    fn issue_type(&self, id: u64) -> Result<IssueType> {
        log::trace!("gh: determining type of issue #{id}");
        let issue = self.get_issue(id)?;
        if issue.labels.iter().any(|l| l == "feature") {
            log::trace!("gh: issue #{id} is a Feature");
            Ok(IssueType::Feature)
        } else {
            log::trace!("gh: issue #{id} is a Ticket");
            Ok(IssueType::Ticket)
        }
    }
}

#[cfg(test)]
mod issue_tracker_tests {
    use super::test_support::adapter;
    use super::*;

    #[test]
    fn get_issue_parses_response() {
        let json = r#"{
            "number": 42,
            "title": "Add user authentication",
            "body": "We need auth",
            "labels": [{"name": "feature"}, {"name": "aft"}]
        }"#;
        let (adapter, _) = adapter(json);
        let issue = adapter.get_issue(42).unwrap();
        assert_eq!(issue.id, 42);
        assert_eq!(issue.title, "Add user authentication");
        assert_eq!(issue.body, "We need auth");
        assert_eq!(issue.labels, vec!["feature", "aft"]);
    }

    #[test]
    fn get_children_parses_sub_issues() {
        let json = r#"[
            {"number": 10, "title": "Sub one", "body": "body", "labels": []},
            {"number": 11, "title": "Sub two", "body": "body", "labels": [{"name": "aft"}]}
        ]"#;
        let (adapter, _) = adapter(json);
        let issues = adapter.get_children(5).unwrap();
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].id, 10);
        assert_eq!(issues[1].labels, vec!["aft"]);
    }

    #[test]
    fn get_issues_by_label_parses_list() {
        let json = r#"[
            {"number": 1, "title": "First", "body": "body one", "labels": [{"name": "aft"}]},
            {"number": 2, "title": "Second", "body": "body two", "labels": [{"name": "aft"}, {"name": "bug"}]}
        ]"#;
        let (adapter, _) = adapter(json);
        let issues = adapter.get_issues_by_label("aft").unwrap();
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].id, 1);
        assert_eq!(issues[1].labels, vec!["aft", "bug"]);
    }

    #[test]
    fn claim_issue_posts_comment() {
        let (adapter, calls) = adapter("");
        adapter.claim_issue(42).unwrap();
        let calls = calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0][0], "gh");
        assert_eq!(calls[0][1], "issue");
        assert_eq!(calls[0][2], "comment");
        assert_eq!(calls[0][3], "42");
        assert!(calls[0].contains(&"--repo".to_string()));
        assert!(calls[0].contains(&"owner/repo".to_string()));
        assert!(calls[0].contains(&"--body".to_string()));
    }

    #[test]
    fn complete_issue_adds_label() {
        let (adapter, calls) = adapter("");
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
        let (adapter, calls) = adapter("");
        adapter.skip_issue(9).unwrap();
        let calls = calls.borrow();
        assert_eq!(calls[0][2], "edit");
        assert_eq!(calls[0][3], "9");
        assert!(calls[0].contains(&"--add-label".to_string()));
        assert!(calls[0].contains(&"hitl".to_string()));
    }

    #[test]
    fn post_comment_sends_body() {
        let (adapter, calls) = adapter("");
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
        let (adapter, calls) = adapter(url);
        let issue = adapter
            .create_child_issue(1, "Child title", "Child body")
            .unwrap();
        let calls = calls.borrow();
        assert!(calls[0].contains(&"create".to_string()));
        assert!(calls[0].contains(&"Child title".to_string()));
        assert!(calls[0].contains(&"Child body".to_string()));
        assert!(calls[1].contains(&"api".to_string()));
        assert!(calls[1].contains(&"/repos/owner/repo/issues/1/sub_issues".to_string()));
        assert!(calls[1].contains(&"sub_issue_id=99".to_string()));
        drop(calls);
        assert_eq!(issue.id, 99);
        assert_eq!(issue.title, "Child title");
        assert_eq!(issue.body, "Child body");
    }

    #[test]
    fn issue_type_returns_feature_when_labeled_feature() {
        let json = r#"{"number": 1, "title": "T", "body": "B", "labels": [{"name": "feature"}]}"#;
        let (adapter, _) = adapter(json);
        assert!(matches!(adapter.issue_type(1).unwrap(), IssueType::Feature));
    }

    #[test]
    fn issue_type_returns_ticket_when_no_feature_label() {
        let json = r#"{"number": 1, "title": "T", "body": "B", "labels": [{"name": "bug"}]}"#;
        let (adapter, _) = adapter(json);
        assert!(matches!(adapter.issue_type(1).unwrap(), IssueType::Ticket));
    }
}

impl RemoteClient for GithubAdapter {
    fn create_pr(&self, title: &str, body: &str, branch: &str) -> Result<String> {
        log::info!("gh: creating PR from '{branch}': {title}");
        let url = self.runner.run(
            "gh",
            &[
                "pr", "create", "--repo", &self.repo, "--title", title, "--body", body, "--head",
                branch,
            ],
        )?;
        let url = url.trim().to_string();
        log::info!("gh: PR created — {url}");
        Ok(url)
    }
}

#[cfg(test)]
mod remote_client_tests {
    use super::test_support::adapter;
    use super::*;

    #[test]
    fn create_pr_returns_url() {
        let url = "https://github.com/owner/repo/pull/5\n";
        let (adapter, calls) = adapter(url);
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
}

#[cfg(test)]
mod test_support {
    use super::*;
    use crate::test_utils::fake_runner;
    use std::cell::RefCell;
    use std::rc::Rc;

    pub fn adapter(response: &str) -> (GithubAdapter, Rc<RefCell<Vec<Vec<String>>>>) {
        let (runner, calls) = fake_runner(response);
        (GithubAdapter::new("owner/repo", Box::new(runner)), calls)
    }
}
