use anyhow::Result;

use crate::traits::{AgentOutput, AgentRunner, CommandRunner, RunConfig};

/// Runs a Claude Code agent as a local subprocess.
pub struct LocalRunner {
    runner: Box<dyn CommandRunner>,
    /// Path to the Claude Code settings file passed via --settings.
    settings_file: Option<String>,
}

impl LocalRunner {
    pub fn new(runner: Box<dyn CommandRunner>, settings_file: Option<String>) -> Self {
        Self {
            runner,
            settings_file,
        }
    }
}

impl AgentRunner for LocalRunner {
    fn run(&self, prompt: &str, config: &RunConfig) -> Result<AgentOutput> {
        if config.dry_run {
            log::info!("dry run — skipping agent invocation");
            return Ok(AgentOutput {
                stdout: String::new(),
                success: true,
            });
        }

        let mut args = vec!["--print", "--dangerously-skip-permissions"];

        if let Some(ref settings) = self.settings_file {
            log::trace!("runner: using settings file '{settings}'");
            args.push("--settings");
            args.push(settings.as_str());
        }

        args.push("-p");
        args.push(prompt);

        log::debug!("runner: invoking claude ({} char prompt)", prompt.len());

        match self.runner.run("claude", &args) {
            Ok(stdout) => {
                log::debug!(
                    "runner: claude succeeded ({} chars of output)",
                    stdout.len()
                );
                Ok(AgentOutput {
                    stdout,
                    success: true,
                })
            }
            Err(e) => {
                log::info!("runner: claude invocation failed: {e}");
                Ok(AgentOutput {
                    stdout: String::new(),
                    success: false,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{fake_failing_runner, fake_runner};
    use crate::traits::MergeStrategy;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn run_config(dry_run: bool) -> RunConfig {
        RunConfig {
            max_iterations: 10,
            merge_strategy: MergeStrategy::Direct,
            base_branch: "main".to_string(),
            use_worktree: false,
            dry_run,
            repo_context: String::new(),
            work_directory: std::path::PathBuf::from("."),
        }
    }

    fn runner(response: &str) -> (LocalRunner, Rc<RefCell<Vec<Vec<String>>>>) {
        let (fake, calls) = fake_runner(response);
        (LocalRunner::new(Box::new(fake), None), calls)
    }

    fn runner_with_settings(settings_file: &str) -> (LocalRunner, Rc<RefCell<Vec<Vec<String>>>>) {
        let (fake, calls) = fake_runner("");
        (
            LocalRunner::new(Box::new(fake), Some(settings_file.to_string())),
            calls,
        )
    }

    #[test]
    fn run_calls_claude_with_prompt() {
        let (runner, calls) = runner("");

        runner.run("do the thing", &run_config(false)).unwrap();

        let calls = calls.borrow();
        assert_eq!(calls[0][0], "claude");
        assert!(calls[0].contains(&"--print".to_string()));
        assert!(calls[0].contains(&"--dangerously-skip-permissions".to_string()));
        assert!(calls[0].contains(&"-p".to_string()));
        assert!(calls[0].contains(&"do the thing".to_string()));
    }

    #[test]
    fn run_with_settings_file_includes_settings_flag() {
        let (runner, calls) = runner_with_settings("settings.agent.json");

        runner.run("do the thing", &run_config(false)).unwrap();

        let calls = calls.borrow();
        assert!(calls[0].contains(&"--settings".to_string()));
        assert!(calls[0].contains(&"settings.agent.json".to_string()));
    }

    #[test]
    fn run_returns_stdout_as_agent_output() {
        let (runner, _) = runner("agent finished successfully");

        let output = runner.run("do the thing", &run_config(false)).unwrap();

        assert_eq!(output.stdout, "agent finished successfully");
        assert!(output.success);
    }

    #[test]
    fn run_dry_run_skips_execution() {
        let (runner, calls) = runner("");

        let output = runner.run("do the thing", &run_config(true)).unwrap();

        assert!(calls.borrow().is_empty());
        assert!(output.success);
    }

    #[test]
    fn run_failed_command_returns_unsuccessful_output() {
        let (fake, _) = fake_failing_runner();
        let runner = LocalRunner::new(Box::new(fake), None);

        let output = runner.run("do the thing", &run_config(false)).unwrap();

        assert!(!output.success);
    }
}
