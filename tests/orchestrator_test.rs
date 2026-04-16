use intern::cli::Command;
use intern::config::{AgentConfig, Config, IssueTrackerConfig, RunDefaults, SourceControlConfig};
use intern::orchestrator::run;

fn implement_command(issue_id: u64) -> Command {
    Command::Implement {
        issue_id,
        dry_run: false,
        max_iterations: None,
        merge_strategy: None,
    }
}

fn github_config() -> Config {
    Config {
        issue_tracker: IssueTrackerConfig {
            kind: "github".to_string(),
            repo: "owner/repo".to_string(),
        },
        agent: AgentConfig {
            kind: "local".to_string(),
            settings_file: None,
        },
        run: RunDefaults::default(),
        source_control: SourceControlConfig::default(),
        context_file: None,
        work_directory: None,
    }
}

#[test]
fn run_returns_error_for_unknown_issue_tracker_kind() {
    let mut config = github_config();
    config.issue_tracker.kind = "linear".to_string();

    let result = run(implement_command(1), config);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("linear"));
}

#[test]
fn run_returns_error_for_unknown_agent_kind() {
    let mut config = github_config();
    config.agent.kind = "docker".to_string();

    let result = run(implement_command(1), config);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("docker"));
}
