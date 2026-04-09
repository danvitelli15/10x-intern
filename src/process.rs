use anyhow::{Context, Result};
use std::process::Command;

use crate::traits::CommandRunner;

/// Production CommandRunner — executes real subprocesses.
pub struct ProcessRunner;

impl CommandRunner for ProcessRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        let output = Command::new(program)
            .args(args)
            .output()
            .with_context(|| format!("failed to run `{program}`"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("`{program}` exited with error: {stderr}");
        }

        String::from_utf8(output.stdout).context("command output was not valid UTF-8")
    }
}
