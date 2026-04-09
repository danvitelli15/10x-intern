use anyhow::{Context, Result};
use std::process::Command;

/// Run a `gh` CLI command and return its stdout as a String.
pub fn run_gh(args: &[&str]) -> Result<String> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .context("failed to run gh CLI — is it installed and on PATH?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh exited with error: {}", stderr);
    }

    Ok(String::from_utf8(output.stdout).context("gh output was not valid UTF-8")?)
}
