use anyhow::Result;

// --- Domain types ---

#[derive(Debug, Clone)]
pub struct Issue {
    pub id: u64,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum CommitStrategy {
    Direct,
    FeatureBranch,
    PerTicket,
}

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub max_iterations: u32,
    pub commit_strategy: CommitStrategy,
    pub dry_run: bool,
}

#[derive(Debug)]
pub struct AgentOutput {
    pub stdout: String,
    pub success: bool,
}

#[derive(Debug)]
pub enum Event {
    IssueClaimed(u64),
    IssueComplete(u64),
    AgentStarted(u64),
    AgentFinished { issue_id: u64, success: bool },
    ReviewStarted,
    ReviewComplete { issues_created: usize },
    RunComplete,
}

// --- Ports (traits) ---

pub trait IssueTracker {
    fn get_issue(&self, id: u64) -> Result<Issue>;
    fn get_children(&self, id: u64) -> Result<Vec<Issue>>;
    fn get_issues_by_label(&self, label: &str) -> Result<Vec<Issue>>;
    fn claim_issue(&self, id: u64) -> Result<()>;
    fn complete_issue(&self, id: u64) -> Result<()>;
    fn skip_issue(&self, id: u64) -> Result<()>;
    fn post_comment(&self, id: u64, body: &str) -> Result<()>;
    fn create_child_issue(&self, parent_id: u64, title: &str, body: &str) -> Result<Issue>;
}

pub trait SourceControl {
    fn create_branch(&self, name: &str) -> Result<()>;
    fn current_branch(&self) -> Result<String>;
    fn diff_from_base(&self, base: &str) -> Result<String>;
    /// Stage files before committing.
    /// `None` stages everything (`git add -A`).
    /// `Some(paths)` stages only the specified paths.
    fn stage(&self, paths: Option<&[&str]>) -> Result<()>;
    fn commit(&self, message: &str) -> Result<()>;
}

pub trait RemoteClient {
    /// Returns the URL of the created PR.
    fn create_pr(&self, title: &str, body: &str, branch: &str) -> Result<String>;
}

pub trait AgentRunner {
    fn run(&self, prompt: &str, config: &RunConfig) -> Result<AgentOutput>;
}

pub trait EventSink {
    fn emit(&self, event: Event);
}

pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String>;
}
