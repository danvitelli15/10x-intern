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
pub enum CommitStrategy {
    Direct,
    PerTicket,
    FeatureBranch,
}

impl CommitStrategy {
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
            Self::PerTicket => "Create a commit per ticket on the current branch",
            Self::FeatureBranch => "Create a dedicated branch per feature",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::Direct, Self::PerTicket, Self::FeatureBranch]
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::all().iter().find(|v| v.key() == key).copied()
    }
}

impl std::fmt::Display for CommitStrategy {
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

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub max_iterations: u32,
    pub commit_strategy: CommitStrategy,
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
