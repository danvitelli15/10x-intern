use anyhow::Result;

use crate::behaviors::{complete_ticket, detect_wizard_hints, execute_ordered, interactive_config_wizard, scaffold_intern_directory, UserInteractor, WizardOutput};
use crate::cli::{Command, MergeStrategyArg};
use crate::config::Config;
use crate::context::Context;
use crate::git::GitClient;
use crate::github::GithubAdapter;
use crate::process::ProcessRunner;
use crate::reporter::log_reporter::LogReporter;
use crate::runner::LocalRunner;
use crate::traits::{MergeStrategy, RunConfig};

pub fn init_workflow(base_dir: &std::path::Path, interactor: &dyn UserInteractor) -> Result<()> {
    let runner = ProcessRunner;
    let hints = detect_wizard_hints(base_dir, &runner);
    let wizard_output = interactive_config_wizard(base_dir, interactor, &hints)?;
    scaffold_intern_directory(base_dir, &wizard_output)
}

pub fn init_workflow_with_defaults(base_dir: &std::path::Path) -> Result<()> {
    scaffold_intern_directory(base_dir, &WizardOutput::defaults())
}

pub fn implement_workflow(issue_id: u64, ctx: &Context) -> Result<()> {
    log::info!("starting implement for issue #{issue_id}");
    complete_ticket(issue_id, ctx, &ctx.config.base_branch.clone())
}

pub fn clear_workflow(label: &str, ctx: &Context) -> Result<()> {
    log::info!("fetching issues with label '{label}'");
    let issues = ctx.issues.get_issues_by_label(label)?;
    log::info!("found {} issue(s) to process", issues.len());
    execute_ordered(&issues, ctx, &ctx.config.base_branch.clone())
}

pub fn build_context(command: &Command, config: &Config) -> Result<Context> {
    let issues = match config.issue_tracker.kind.as_str() {
        "github" => Box::new(GithubAdapter::new(
            &config.issue_tracker.repo,
            Box::new(ProcessRunner),
        )) as Box<dyn crate::traits::IssueTracker>,
        kind => anyhow::bail!("unknown issue_tracker.kind: {kind}"),
    };

    let source_control = Box::new(GitClient::new(Box::new(ProcessRunner))) as Box<dyn crate::traits::SourceControl>;

    let remote = match config.issue_tracker.kind.as_str() {
        "github" => Box::new(GithubAdapter::new(
            &config.issue_tracker.repo,
            Box::new(ProcessRunner),
        )) as Box<dyn crate::traits::RemoteClient>,
        kind => anyhow::bail!("unknown remote kind: {kind}"),
    };

    let runner = match config.agent.kind.as_str() {
        "local" => Box::new(LocalRunner::new(
            Box::new(ProcessRunner),
            config.agent.settings_file.clone(),
        )) as Box<dyn crate::traits::AgentRunner>,
        kind => anyhow::bail!("unknown agent.kind: {kind}"),
    };

    let events = Box::new(LogReporter) as Box<dyn crate::traits::EventSink>;

    Ok(Context::new(issues, source_control, remote, runner, events, build_run_config(command, config)?))
}

pub fn build_run_config(command: &Command, config: &Config) -> Result<RunConfig> {
    let (dry_run, max_iterations_override, merge_strategy_override) = match command {
        Command::Implement { dry_run, max_iterations, merge_strategy, .. } => {
            (*dry_run, *max_iterations, merge_strategy.clone())
        }
        Command::Clear { dry_run, max_iterations, merge_strategy, .. } => {
            (*dry_run, *max_iterations, merge_strategy.clone())
        }
        Command::Review { dry_run, .. } => (*dry_run, None, None),
        Command::Init { .. } => unreachable!(),
    };

    let merge_strategy = match merge_strategy_override {
        Some(MergeStrategyArg::Direct) => MergeStrategy::Direct,
        Some(MergeStrategyArg::PerTicket) => MergeStrategy::PerTicket,
        Some(MergeStrategyArg::FeatureBranch) => MergeStrategy::FeatureBranch,
        None => MergeStrategy::from_key(&config.source_control.merge_strategy)
            .unwrap_or(MergeStrategy::FeatureBranch),
    };

    Ok(RunConfig {
        max_iterations: max_iterations_override.unwrap_or(config.run.max_iterations),
        merge_strategy,
        base_branch: config.source_control.base_branch.clone(),
        use_worktree: config.source_control.use_worktree,
        dry_run,
        repo_context: config.resolve_repo_context()?,
        work_directory: config.resolve_work_directory(),
    })
}

#[cfg(test)]
mod build_run_config_tests {
    use super::*;
    use crate::config::{AgentConfig, IssueTrackerConfig, RunDefaults, SourceControlConfig};

    fn config_with_source_control(sc: SourceControlConfig) -> Config {
        Config {
            issue_tracker: IssueTrackerConfig { kind: "github".into(), repo: "o/r".into() },
            agent: AgentConfig { kind: "local".into(), settings_file: None },
            run: RunDefaults::default(),
            source_control: sc,
            context_file: None,
            work_directory: None,
        }
    }

    fn implement(merge_strategy: Option<MergeStrategyArg>) -> Command {
        Command::Implement { issue_id: 1, dry_run: false, max_iterations: None, merge_strategy }
    }

    #[test]
    fn build_run_config_reads_merge_strategy_from_source_control() {
        let config = config_with_source_control(SourceControlConfig {
            merge_strategy: "per-ticket".to_string(),
            ..SourceControlConfig::default()
        });
        let run_config = build_run_config(&implement(None), &config).unwrap();
        assert_eq!(run_config.merge_strategy, MergeStrategy::PerTicket);
    }

    #[test]
    fn build_run_config_reads_base_branch_from_source_control() {
        let config = config_with_source_control(SourceControlConfig {
            base_branch: "develop".to_string(),
            ..SourceControlConfig::default()
        });
        let run_config = build_run_config(&implement(None), &config).unwrap();
        assert_eq!(run_config.base_branch, "develop");
    }

    #[test]
    fn build_run_config_cli_flag_overrides_config_merge_strategy() {
        let config = config_with_source_control(SourceControlConfig {
            merge_strategy: "feature-branch".to_string(),
            ..SourceControlConfig::default()
        });
        let run_config = build_run_config(&implement(Some(MergeStrategyArg::Direct)), &config).unwrap();
        assert_eq!(run_config.merge_strategy, MergeStrategy::Direct);
    }
}
