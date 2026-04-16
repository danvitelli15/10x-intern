use anyhow::Result;
use figment::{
    Figment,
    providers::{Format, Toml},
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub issue_tracker: IssueTrackerConfig,
    pub agent: AgentConfig,
    #[serde(default)]
    pub run: RunDefaults,
    #[serde(default)]
    pub source_control: SourceControlConfig,
    pub context_file: Option<String>,
    pub work_directory: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IssueTrackerConfig {
    /// Adapter kind — currently only "github" is supported.
    pub kind: String,
    /// Repository in "owner/repo" format.
    pub repo: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    /// Runner kind — currently only "local" is supported.
    pub kind: String,
    /// Path to the Claude Code settings file passed via --settings.
    pub settings_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SourceControlConfig {
    pub kind: String,
    pub base_branch: String,
    pub merge_strategy: String,
    pub use_worktree: bool,
    pub on_dirty_after_commit: String,
    pub on_dirty_no_commits: String,
}

impl Default for SourceControlConfig {
    fn default() -> Self {
        Self {
            kind: "git".to_string(),
            base_branch: "main".to_string(),
            merge_strategy: "feature-branch".to_string(),
            use_worktree: false,
            on_dirty_after_commit: "warn".to_string(),
            on_dirty_no_commits: "fail".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RunDefaults {
    pub max_iterations: u32,
}

impl Default for RunDefaults {
    fn default() -> Self {
        Self {
            max_iterations: 100,
        }
    }
}

#[cfg(test)]
mod source_control_config_tests {
    use super::*;

    #[test]
    fn source_control_config_defaults_when_section_absent() {
        let toml = r#"
            [issue_tracker]
            kind = "github"
            repo = "owner/repo"
            [agent]
            kind = "local"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.source_control.kind, "git");
        assert_eq!(config.source_control.base_branch, "main");
        assert_eq!(config.source_control.merge_strategy, "feature-branch");
        assert!(!config.source_control.use_worktree);
        assert_eq!(config.source_control.on_dirty_after_commit, "warn");
        assert_eq!(config.source_control.on_dirty_no_commits, "fail");
    }

    #[test]
    fn source_control_config_deserializes_explicit_section() {
        let toml = r#"
            [issue_tracker]
            kind = "github"
            repo = "owner/repo"
            [agent]
            kind = "local"
            [source_control]
            kind = "git"
            base_branch = "trunk"
            merge_strategy = "per-ticket"
            use_worktree = true
            on_dirty_after_commit = "fail"
            on_dirty_no_commits = "warn"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.source_control.kind, "git");
        assert_eq!(config.source_control.base_branch, "trunk");
        assert_eq!(config.source_control.merge_strategy, "per-ticket");
        assert!(config.source_control.use_worktree);
        assert_eq!(config.source_control.on_dirty_after_commit, "fail");
        assert_eq!(config.source_control.on_dirty_no_commits, "warn");
    }
}

#[cfg(test)]
mod config_deserialization_tests {
    use super::*;

    #[test]
    fn config_deserializes_context_file() {
        let toml = r#"
            context_file = "CLAUDE.md"
            [issue_tracker]
            kind = "github"
            repo = "owner/repo"
            [agent]
            kind = "local"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.context_file, Some("CLAUDE.md".to_string()));
    }

    #[test]
    fn config_deserializes_work_directory() {
        let toml = r#"
            work_directory = "/projects/myrepo"
            [issue_tracker]
            kind = "github"
            repo = "owner/repo"
            [agent]
            kind = "local"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.work_directory, Some("/projects/myrepo".to_string()));
    }

    #[test]
    fn config_work_directory_defaults_to_none() {
        let toml = r#"
            [issue_tracker]
            kind = "github"
            repo = "owner/repo"
            [agent]
            kind = "local"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.work_directory, None);
    }

    #[test]
    fn config_context_file_defaults_to_none() {
        let toml = r#"
            [issue_tracker]
            kind = "github"
            repo = "owner/repo"
            [agent]
            kind = "local"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.context_file, None);
    }
}

impl Config {
    pub fn resolve_work_directory(&self) -> std::path::PathBuf {
        match &self.work_directory {
            Some(dir) => std::path::PathBuf::from(dir),
            None => std::env::current_dir().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod resolve_work_directory_tests {
    use super::*;

    fn config(work_directory: Option<&str>) -> Config {
        Config {
            issue_tracker: IssueTrackerConfig {
                kind: "github".into(),
                repo: "o/r".into(),
            },
            agent: AgentConfig {
                kind: "local".into(),
                settings_file: None,
            },
            run: RunDefaults::default(),
            source_control: SourceControlConfig::default(),
            context_file: None,
            work_directory: work_directory.map(Into::into),
        }
    }

    #[test]
    fn resolve_work_directory_uses_config_value_when_set() {
        assert_eq!(
            config(Some("/projects/myrepo")).resolve_work_directory(),
            std::path::PathBuf::from("/projects/myrepo")
        );
    }

    #[test]
    fn resolve_work_directory_defaults_to_cwd_when_not_set() {
        assert_eq!(
            config(None).resolve_work_directory(),
            std::env::current_dir().unwrap()
        );
    }
}

impl Config {
    pub fn resolve_repo_context(&self) -> Result<String> {
        match &self.context_file {
            Some(path) => Ok(std::fs::read_to_string(path)?),
            None => Ok(String::new()),
        }
    }
}

#[cfg(test)]
mod resolve_repo_context_tests {
    use super::*;

    fn config(context_file: Option<&str>) -> Config {
        Config {
            issue_tracker: IssueTrackerConfig {
                kind: "github".into(),
                repo: "o/r".into(),
            },
            agent: AgentConfig {
                kind: "local".into(),
                settings_file: None,
            },
            run: RunDefaults::default(),
            source_control: SourceControlConfig::default(),
            context_file: context_file.map(Into::into),
            work_directory: None,
        }
    }

    #[test]
    fn resolve_repo_context_returns_file_contents_when_context_file_set() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "use snake_case everywhere").unwrap();
        let config = config(Some(file.path().to_str().unwrap()));
        assert_eq!(
            config.resolve_repo_context().unwrap(),
            "use snake_case everywhere"
        );
    }

    #[test]
    fn resolve_repo_context_returns_empty_string_when_not_set() {
        assert_eq!(config(None).resolve_repo_context().unwrap(), "");
    }

    #[test]
    fn resolve_repo_context_errors_when_context_file_missing() {
        assert!(
            config(Some("/nonexistent/path/context.md"))
                .resolve_repo_context()
                .is_err()
        );
    }
}

impl Config {
    /// Load config using the current working directory as the project root.
    pub fn load() -> Result<Self> {
        Self::load_from(&std::env::current_dir()?)
    }

    /// Load config from (in order, later overrides earlier):
    ///   1. ~/.intern/config.toml      (global user config)
    ///   2. <dir>/.intern.toml         (legacy per-repo config)
    ///   3. <dir>/.intern/config.toml  (per-repo config, takes precedence)
    ///
    /// CLI flag overrides are applied in main before constructing adapters.
    pub fn load_from(dir: &std::path::Path) -> Result<Self> {
        let global = dirs::home_dir()
            .map(|h| h.join(".intern/config.toml"))
            .unwrap_or_default();

        let config = Figment::new()
            .merge(Toml::file(global))
            .merge(Toml::file(dir.join(".intern.toml")))
            .merge(Toml::file(dir.join(".intern/config.toml")))
            .extract()?;

        Ok(config)
    }
}

#[cfg(test)]
mod load_from_tests {
    use super::*;

    #[test]
    fn load_from_finds_intern_directory_config() {
        let dir = tempfile::tempdir().unwrap();
        let intern_dir = dir.path().join(".intern");
        std::fs::create_dir_all(&intern_dir).unwrap();
        std::fs::write(
            intern_dir.join("config.toml"),
            r#"
            [issue_tracker]
            kind = "github"
            repo = "owner/repo"
            [agent]
            kind = "local"
        "#,
        )
        .unwrap();
        let config = Config::load_from(dir.path()).unwrap();
        assert_eq!(config.issue_tracker.repo, "owner/repo");
    }

    #[test]
    fn load_from_falls_back_to_legacy_intern_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".intern.toml"),
            r#"
            [issue_tracker]
            kind = "github"
            repo = "legacy/repo"
            [agent]
            kind = "local"
        "#,
        )
        .unwrap();
        let config = Config::load_from(dir.path()).unwrap();
        assert_eq!(config.issue_tracker.repo, "legacy/repo");
    }

    #[test]
    fn load_from_intern_directory_config_takes_precedence_over_legacy() {
        let dir = tempfile::tempdir().unwrap();
        let intern_dir = dir.path().join(".intern");
        std::fs::create_dir_all(&intern_dir).unwrap();
        std::fs::write(
            intern_dir.join("config.toml"),
            r#"
            [issue_tracker]
            kind = "github"
            repo = "new/repo"
            [agent]
            kind = "local"
        "#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join(".intern.toml"),
            r#"
            [issue_tracker]
            kind = "github"
            repo = "legacy/repo"
            [agent]
            kind = "local"
        "#,
        )
        .unwrap();
        let config = Config::load_from(dir.path()).unwrap();
        assert_eq!(config.issue_tracker.repo, "new/repo");
    }
}
