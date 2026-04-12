use anyhow::Result;
use figment::{
    providers::{Format, Toml},
    Figment,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub issue_tracker: IssueTrackerConfig,
    pub agent: AgentConfig,
    #[serde(default)]
    pub run: RunDefaults,
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
pub struct RunDefaults {
    pub max_iterations: u32,
    pub commit_strategy: String,
}

impl Default for RunDefaults {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            commit_strategy: "feature-branch".to_string(),
        }
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
mod tests {
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
    fn resolve_work_directory_uses_config_value_when_set() {
        let config = Config {
            issue_tracker: IssueTrackerConfig { kind: "github".into(), repo: "o/r".into() },
            agent: AgentConfig { kind: "local".into(), settings_file: None },
            run: RunDefaults::default(),
            context_file: None,
            work_directory: Some("/projects/myrepo".into()),
        };
        assert_eq!(config.resolve_work_directory(), std::path::PathBuf::from("/projects/myrepo"));
    }

    #[test]
    fn resolve_work_directory_defaults_to_cwd_when_not_set() {
        let config = Config {
            issue_tracker: IssueTrackerConfig { kind: "github".into(), repo: "o/r".into() },
            agent: AgentConfig { kind: "local".into(), settings_file: None },
            run: RunDefaults::default(),
            context_file: None,
            work_directory: None,
        };
        assert_eq!(config.resolve_work_directory(), std::env::current_dir().unwrap());
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
    /// Load config from (in order, later overrides earlier):
    ///   1. ~/.intern/config.toml  (global user config)
    ///   2. .intern.toml           (per-repo config)
    ///
    /// CLI flag overrides are applied in main before constructing adapters.
    pub fn load() -> Result<Self> {
        let global = dirs::home_dir()
            .map(|h| h.join(".intern/config.toml"))
            .unwrap_or_default();

        let config = Figment::new()
            .merge(Toml::file(global))
            .merge(Toml::file(".intern.toml"))
            .extract()?;

        Ok(config)
    }
}
