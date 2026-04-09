use anyhow::Result;

use crate::traits::{AgentOutput, AgentRunner, RunConfig};

/// Runs a Claude Code agent as a local subprocess.
pub struct LocalRunner {
    /// Path to the Claude Code settings file (--settings flag).
    settings_file: Option<String>,
}

impl LocalRunner {
    pub fn new(settings_file: Option<String>) -> Self {
        Self { settings_file }
    }
}

impl AgentRunner for LocalRunner {
    fn run(&self, prompt: &str, config: &RunConfig) -> Result<AgentOutput> {
        todo!("shell out to: claude --print --dangerously-skip-permissions [-settings <file>] -p <prompt>")
    }
}
