use intern::behaviors::{interactive_config_wizard, WizardHints, WizardOutput};
use intern::traits::{IssueTrackerKind, MergeStrategy, SourceControlKind};
use intern::workflows::{init_workflow, init_workflow_with_defaults};

mod common;

#[test]
fn wizard_captures_source_control_kind() {
    let interactor = common::StubInteractor::new()
        .with_choice(0)          // issue tracker: github
        .with_text("owner/repo") // repo
        .with_choice(1)          // source control: none (index 1)
        .with_choice(0)          // agent: local
        .with_confirm(false)     // settings file: no
        .with_confirm(false)     // context file: no
        .with_choice(0)          // commit strategy: direct
        .with_confirm(true);     // summary

    let dir = tempfile::tempdir().unwrap();
    let output = interactive_config_wizard(dir.path(), &interactor, &WizardHints::none()).unwrap();
    assert_eq!(output.source_control_kind, SourceControlKind::None);
}

#[test]
fn wizard_captures_repo_from_user_input() {
    let interactor = common::StubInteractor::new()
        .with_choice(0)          // issue tracker kind: github
        .with_text("myorg/myrepo")  // repo slug
        .with_choice(0)          // source control: git
        .with_choice(0)          // agent kind: local
        .with_confirm(false)     // settings file: no
        .with_confirm(false)     // context file: no
        .with_choice(0)          // commit strategy: direct
        .with_confirm(true);     // summary: looks good

    let dir = tempfile::tempdir().unwrap();
    let output = interactive_config_wizard(dir.path(), &interactor, &WizardHints::none()).unwrap();
    assert_eq!(output.repo, "myorg/myrepo");
}

#[test]
fn wizard_captures_context_file_when_user_says_yes() {
    let interactor = common::StubInteractor::new()
        .with_choice(0)             // issue tracker: github
        .with_text("owner/repo")    // repo
        .with_choice(0)             // source control: git
        .with_choice(0)             // agent: local
        .with_confirm(false)        // settings file: no
        .with_confirm(true)         // context file: yes
        .with_text("AGENTS.md")     // context file path
        .with_choice(2)             // commit strategy: feature-branch
        .with_confirm(true);        // summary

    let dir = tempfile::tempdir().unwrap();
    let output = interactive_config_wizard(dir.path(), &interactor, &WizardHints::none()).unwrap();
    assert_eq!(output.context_file, Some("AGENTS.md".to_string()));
}

#[test]
fn wizard_context_file_is_none_when_user_says_no() {
    let interactor = common::StubInteractor::new()
        .with_choice(0)             // issue tracker: github
        .with_text("owner/repo")
        .with_choice(0)             // source control: git
        .with_choice(0)             // agent: local
        .with_confirm(false)        // settings file: no
        .with_confirm(false)        // context file: no
        .with_choice(2)
        .with_confirm(true);

    let dir = tempfile::tempdir().unwrap();
    let output = interactive_config_wizard(dir.path(), &interactor, &WizardHints::none()).unwrap();
    assert_eq!(output.context_file, None);
}

#[test]
fn wizard_captures_merge_strategy_selection() {
    let interactor = common::StubInteractor::new()
        .with_choice(0)             // issue tracker: github
        .with_text("owner/repo")
        .with_choice(0)             // source control: git
        .with_choice(0)             // agent: local
        .with_confirm(false)        // settings file: no
        .with_confirm(false)        // context file: no
        .with_choice(1)             // merge strategy: per-ticket (index 1)
        .with_confirm(true);

    let dir = tempfile::tempdir().unwrap();
    let output = interactive_config_wizard(dir.path(), &interactor, &WizardHints::none()).unwrap();
    assert_eq!(output.merge_strategy, MergeStrategy::PerTicket);
}

#[test]
fn wizard_output_defaults_uses_github_issue_tracker() {
    let output = WizardOutput::defaults();
    assert_eq!(output.issue_tracker_kind, IssueTrackerKind::GitHub);
}

#[test]
fn wizard_output_defaults_uses_feature_branch_merge_strategy() {
    let output = WizardOutput::defaults();
    assert_eq!(output.merge_strategy, MergeStrategy::FeatureBranch);
}

#[test]
fn wizard_output_defaults_uses_local_agent() {
    use intern::traits::AgentKind;
    let output = WizardOutput::defaults();
    assert_eq!(output.agent_kind, AgentKind::Local);
}

#[test]
fn scaffold_writes_issue_tracker_values_to_config() {
    let dir = tempfile::tempdir().unwrap();
    let output = WizardOutput { repo: "acme/my-app".to_string(), ..WizardOutput::defaults() };
    intern::behaviors::scaffold_intern_directory(dir.path(), &output).unwrap();
    let config = std::fs::read_to_string(dir.path().join(".intern/config.toml")).unwrap();
    assert!(config.contains("github"));
    assert!(config.contains("acme/my-app"));
}

#[test]
fn scaffold_writes_merge_strategy_to_config() {
    let dir = tempfile::tempdir().unwrap();
    let output = WizardOutput { merge_strategy: MergeStrategy::PerTicket, ..WizardOutput::defaults() };
    intern::behaviors::scaffold_intern_directory(dir.path(), &output).unwrap();
    let config = std::fs::read_to_string(dir.path().join(".intern/config.toml")).unwrap();
    assert!(config.contains("per-ticket"));
}

#[test]
fn scaffold_writes_merge_strategy_and_base_branch_under_source_control_section() {
    let dir = tempfile::tempdir().unwrap();
    intern::behaviors::scaffold_intern_directory(dir.path(), &WizardOutput::defaults()).unwrap();
    let raw = std::fs::read_to_string(dir.path().join(".intern/config.toml")).unwrap();
    // [source_control] section must come before [run] and contain merge_strategy and base_branch
    let sc_pos = raw.find("[source_control]").expect("[source_control] section missing");
    let run_pos = raw.find("[run]").expect("[run] section missing");
    assert!(sc_pos < run_pos, "[source_control] should appear before [run]");
    let sc_section = &raw[sc_pos..run_pos];
    assert!(sc_section.contains("merge_strategy"), "merge_strategy not in [source_control]");
    assert!(sc_section.contains("base_branch"), "base_branch not in [source_control]");
    // [run] section must NOT contain merge_strategy
    let run_section = &raw[run_pos..];
    assert!(!run_section.contains("merge_strategy"), "merge_strategy must not appear in [run]");
}

#[test]
fn scaffold_writes_context_file_when_set() {
    let dir = tempfile::tempdir().unwrap();
    let output = WizardOutput { context_file: Some("CLAUDE.md".to_string()), ..WizardOutput::defaults() };
    intern::behaviors::scaffold_intern_directory(dir.path(), &output).unwrap();
    let config = std::fs::read_to_string(dir.path().join(".intern/config.toml")).unwrap();
    assert!(config.contains("CLAUDE.md"));
}

#[test]
fn scaffold_omits_context_file_when_not_set() {
    let dir = tempfile::tempdir().unwrap();
    let output = WizardOutput { context_file: None, ..WizardOutput::defaults() };
    intern::behaviors::scaffold_intern_directory(dir.path(), &output).unwrap();
    let config = std::fs::read_to_string(dir.path().join(".intern/config.toml")).unwrap();
    assert!(!config.contains("context_file"));
}

#[test]
fn init_workflow_with_defaults_creates_config_file() {
    let dir = tempfile::tempdir().unwrap();
    init_workflow_with_defaults(dir.path()).unwrap();
    assert!(dir.path().join(".intern/config.toml").exists());
}

#[test]
fn init_workflow_with_defaults_creates_all_prompt_files() {
    let dir = tempfile::tempdir().unwrap();
    init_workflow_with_defaults(dir.path()).unwrap();
    for name in &["implement", "review", "feature_review", "plan_order", "test_instructions"] {
        assert!(dir.path().join(format!(".intern/prompts/{name}.md")).exists(), "{name}.md missing");
    }
}

#[test]
fn init_workflow_with_defaults_errors_if_already_initialized() {
    let dir = tempfile::tempdir().unwrap();
    init_workflow_with_defaults(dir.path()).unwrap();
    assert!(init_workflow_with_defaults(dir.path()).is_err());
}

#[test]
fn init_workflow_runs_wizard_and_writes_its_output_to_config() {
    let interactor = common::StubInteractor::new()
        .with_choice(0)                 // issue tracker: github
        .with_text("acme/widgets")      // repo
        .with_choice(0)                 // source control: git
        .with_choice(0)                 // agent: local
        .with_confirm(false)            // settings file: no
        .with_confirm(false)            // context file: no
        .with_choice(0)                 // commit strategy: direct
        .with_confirm(true);            // summary

    let dir = tempfile::tempdir().unwrap();
    init_workflow(dir.path(), &interactor).unwrap();

    let config = std::fs::read_to_string(dir.path().join(".intern/config.toml")).unwrap();
    assert!(config.contains("acme/widgets"));
    assert!(config.contains("direct"));
}
