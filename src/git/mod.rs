use anyhow::Result;

use crate::traits::{CommandRunner, VcsClient};

pub struct GitClient {
    runner: Box<dyn CommandRunner>,
}

impl GitClient {
    pub fn new(runner: Box<dyn CommandRunner>) -> Self {
        Self { runner }
    }
}

impl VcsClient for GitClient {
    fn create_branch(&self, name: &str) -> Result<()> {
        self.runner.run("git", &["switch", "-c", name])?;
        Ok(())
    }

    fn current_branch(&self) -> Result<String> {
        let output = self
            .runner
            .run("git", &["rev-parse", "--abbrev-ref", "HEAD"])?;
        Ok(output.trim().to_string())
    }

    fn diff_from_main(&self) -> Result<String> {
        self.runner.run("git", &["diff", "main...HEAD"])
    }

    fn stage(&self, paths: Option<&[&str]>) -> Result<()> {
        match paths {
            None => self.runner.run("git", &["add", "-A"])?,
            Some(paths) => {
                let mut args = vec!["add"];
                args.extend_from_slice(paths);
                self.runner.run("git", &args)?
            }
        };
        Ok(())
    }

    fn commit(&self, message: &str) -> Result<()> {
        self.runner.run("git", &["commit", "-m", message])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::fake_runner;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn adapter(response: &str) -> (GitClient, Rc<RefCell<Vec<Vec<String>>>>) {
        let (runner, calls) = fake_runner(response);
        (GitClient::new(Box::new(runner)), calls)
    }

    #[test]
    fn create_branch_runs_git_switch() {
        let (client, calls) = adapter("");

        client.create_branch("feature/123").unwrap();

        let calls = calls.borrow();
        assert_eq!(calls[0][0], "git");
        assert_eq!(calls[0][1], "switch");
        assert_eq!(calls[0][2], "-c");
        assert_eq!(calls[0][3], "feature/123");
    }

    #[test]
    fn current_branch_returns_trimmed_output() {
        let (client, _) = adapter("feature/123\n");

        let branch = client.current_branch().unwrap();

        assert_eq!(branch, "feature/123");
    }

    #[test]
    fn diff_from_main_returns_output() {
        let diff = "diff --git a/src/main.rs b/src/main.rs\n+fn hello() {}";
        let (client, _) = adapter(diff);

        let result = client.diff_from_main().unwrap();

        assert_eq!(result, diff);
    }

    #[test]
    fn stage_none_stages_all_files() {
        let (client, calls) = adapter("");

        client.stage(None).unwrap();

        let calls = calls.borrow();
        assert_eq!(calls[0][..], ["git", "add", "-A"]);
    }

    #[test]
    fn stage_some_stages_specific_paths() {
        let (client, calls) = adapter("");

        client.stage(Some(&["src/main.rs", "Cargo.toml"])).unwrap();

        let calls = calls.borrow();
        assert_eq!(calls[0][..], ["git", "add", "src/main.rs", "Cargo.toml"]);
    }

    #[test]
    fn commit_commits_with_message() {
        let (client, calls) = adapter("");

        client.commit("fix: resolve issue #42").unwrap();

        let calls = calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0][..],
            ["git", "commit", "-m", "fix: resolve issue #42"]
        );
    }
}
