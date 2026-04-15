use std::path::Path;

use anyhow::Result;

use crate::traits::CommandRunner;

pub fn detect_repo_slug(runner: &dyn CommandRunner) -> Option<String> {
    log::trace!("detect_repo_slug: checking if gh is available");
    runner.run("gh", &["--version"]).ok()?;
    log::trace!("detect_repo_slug: fetching repo slug via gh");
    let output = runner
        .run(
            "gh",
            &[
                "repo",
                "view",
                "--json",
                "nameWithOwner",
                "--jq",
                ".nameWithOwner",
            ],
        )
        .ok()?;
    let slug = output.trim().to_string();
    log::debug!("detect_repo_slug: detected '{slug}'");
    Some(slug)
}

pub fn find_file(base_dir: &Path, name: &str) -> Option<std::path::PathBuf> {
    let path = base_dir.join(name);
    if path.exists() { Some(path) } else { None }
}

pub fn create_file(path: &Path, content: &str) -> Result<()> {
    log::trace!("create_file: {}", path.display());
    if path.exists() {
        anyhow::bail!("file already exists: {}", path.display());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    struct SequencedFakeRunner {
        responses: std::cell::RefCell<std::collections::VecDeque<Result<String>>>,
    }

    impl SequencedFakeRunner {
        fn new() -> Self {
            Self {
                responses: std::cell::RefCell::new(std::collections::VecDeque::new()),
            }
        }
        fn then(self, result: Result<String>) -> Self {
            self.responses.borrow_mut().push_back(result);
            self
        }
    }

    impl CommandRunner for SequencedFakeRunner {
        fn run(&self, _program: &str, _args: &[&str]) -> Result<String> {
            self.responses
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Err(anyhow!("SequencedFakeRunner: no more responses")))
        }
    }

    #[test]
    fn detect_repo_slug_returns_slug_when_gh_available() {
        let runner = SequencedFakeRunner::new()
            .then(Ok("2.0.0\n".to_string()))
            .then(Ok("acme/widgets\n".to_string()));
        assert_eq!(detect_repo_slug(&runner), Some("acme/widgets".to_string()));
    }

    #[test]
    fn detect_repo_slug_returns_none_when_gh_not_installed() {
        let runner = SequencedFakeRunner::new().then(Err(anyhow!("command not found: gh")));
        assert_eq!(detect_repo_slug(&runner), None);
    }

    #[test]
    fn detect_repo_slug_returns_none_when_gh_repo_view_fails() {
        let runner = SequencedFakeRunner::new()
            .then(Ok("2.0.0\n".to_string()))
            .then(Err(anyhow!("not a git repo")));
        assert_eq!(detect_repo_slug(&runner), None);
    }

    #[test]
    fn create_file_writes_content_to_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output.txt");
        create_file(&path, "hello").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn create_file_errors_if_file_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output.txt");
        std::fs::write(&path, "existing").unwrap();
        assert!(create_file(&path, "new content").is_err());
    }

    #[test]
    fn find_file_returns_path_when_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "").unwrap();
        assert!(find_file(dir.path(), "CLAUDE.md").is_some());
    }

    #[test]
    fn find_file_returns_none_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(find_file(dir.path(), "CLAUDE.md").is_none());
    }

    #[test]
    fn find_file_works_for_directories() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        assert!(find_file(dir.path(), ".git").is_some());
    }

    #[test]
    fn create_file_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/c/output.txt");
        create_file(&path, "hello").unwrap();
        assert!(path.exists());
    }
}
