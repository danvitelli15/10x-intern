use anyhow::Result;

// --- Domain types ---

#[derive(Debug, Clone)]
pub struct Issue {
    pub id: u64,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceControlKind {
    Git,
    None,
}

impl SourceControlKind {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::None => "none",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Git => "Git",
            Self::None => "None",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Git => "Version control via git",
            Self::None => "Skip version control — changes are not committed",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::Git, Self::None]
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::all().iter().find(|v| v.key() == key).copied()
    }
}

impl std::fmt::Display for SourceControlKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} — {}", self.label(), self.description())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    Direct,
    PerTicket,
    FeatureBranch,
}

impl MergeStrategy {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::PerTicket => "per-ticket",
            Self::FeatureBranch => "feature-branch",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Direct => "Direct",
            Self::PerTicket => "Per Ticket",
            Self::FeatureBranch => "Feature Branch",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Direct => "Commit directly to the current branch",
            Self::PerTicket => "Create a branch and PR per ticket",
            Self::FeatureBranch => "All tickets share one branch and one PR",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::Direct, Self::PerTicket, Self::FeatureBranch]
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::all().iter().find(|v| v.key() == key).copied()
    }
}

impl std::fmt::Display for MergeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} — {}", self.label(), self.description())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueTrackerKind {
    GitHub,
}

impl IssueTrackerKind {
    pub fn key(&self) -> &'static str {
        match self {
            Self::GitHub => "github",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::GitHub => "GitHub Issues via the gh CLI",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::GitHub]
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::all().iter().find(|v| v.key() == key).copied()
    }
}

impl std::fmt::Display for IssueTrackerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} — {}", self.label(), self.description())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Local,
}

impl AgentKind {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Local => "local",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Local => "Local (Claude Code)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Local => "Run Claude Code locally via the claude CLI",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::Local]
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::all().iter().find(|v| v.key() == key).copied()
    }
}

impl std::fmt::Display for AgentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} — {}", self.label(), self.description())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirtyBehavior {
    Fail,
    Warn,
    Commit,
}

impl DirtyBehavior {
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "fail" => Some(Self::Fail),
            "warn" => Some(Self::Warn),
            "commit" => Some(Self::Commit),
            _ => None,
        }
    }
}

pub struct RunConfig {
    pub max_iterations: u32,
    pub merge_strategy: MergeStrategy,
    pub base_branch: String,
    pub use_worktree: bool,
    pub on_dirty_after_commit: DirtyBehavior,
    pub on_dirty_no_commits: DirtyBehavior,
    pub dry_run: bool,
    pub repo_context: String,
    pub work_directory: std::path::PathBuf,
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

pub enum IssueType {
    Feature,
    Ticket,
}

pub trait IssueTracker {
    fn get_issue(&self, id: u64) -> Result<Issue>;
    fn get_children(&self, id: u64) -> Result<Vec<Issue>>;
    fn get_issues_by_label(&self, label: &str) -> Result<Vec<Issue>>;
    fn claim_issue(&self, id: u64) -> Result<()>;
    fn complete_issue(&self, id: u64) -> Result<()>;
    fn skip_issue(&self, id: u64) -> Result<()>;
    fn post_comment(&self, id: u64, body: &str) -> Result<()>;
    fn create_child_issue(&self, parent_id: u64, title: &str, body: &str) -> Result<Issue>;
    fn issue_type(&self, id: u64) -> Result<IssueType>;
}

pub trait SourceControl {
    fn create_branch(&self, name: &str, from: &str) -> Result<()>;
    fn current_branch(&self) -> Result<String>;
    fn diff_from_base(&self, base: &str) -> Result<String>;
    /// Returns true if there are uncommitted changes in the working tree.
    fn has_uncommitted_changes(&self) -> Result<bool>;
    /// Returns true if there are commits on the current branch not present in `base`.
    fn has_commits_since(&self, base: &str) -> Result<bool>;
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
