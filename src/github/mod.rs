mod client;
pub mod types;

use anyhow::Result;

use crate::traits::{Issue, IssueTracker, RemoteClient};

/// Adapter implementing both IssueTracker and RemoteClient via the `gh` CLI.
pub struct GithubAdapter {
    repo: String,
}

impl GithubAdapter {
    pub fn new(repo: &str) -> Self {
        Self {
            repo: repo.to_string(),
        }
    }
}

impl IssueTracker for GithubAdapter {
    fn get_issue(&self, id: u64) -> Result<Issue> {
        todo!()
    }

    fn get_children(&self, id: u64) -> Result<Vec<Issue>> {
        todo!()
    }

    fn get_issues_by_label(&self, label: &str) -> Result<Vec<Issue>> {
        todo!()
    }

    fn claim_issue(&self, id: u64) -> Result<()> {
        todo!()
    }

    fn complete_issue(&self, id: u64) -> Result<()> {
        todo!()
    }

    fn skip_issue(&self, id: u64) -> Result<()> {
        todo!()
    }

    fn post_comment(&self, id: u64, body: &str) -> Result<()> {
        todo!()
    }

    fn create_child_issue(&self, parent_id: u64, title: &str, body: &str) -> Result<Issue> {
        todo!()
    }
}

impl RemoteClient for GithubAdapter {
    fn create_pr(&self, title: &str, body: &str, branch: &str) -> Result<String> {
        todo!()
    }
}
