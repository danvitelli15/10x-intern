use serde::Deserialize;

/// Raw shape of a GitHub issue as returned by `gh issue view --json`.
#[derive(Debug, Deserialize)]
pub struct GhIssue {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub labels: Vec<GhLabel>,
}

#[derive(Debug, Deserialize)]
pub struct GhLabel {
    pub name: String,
}
