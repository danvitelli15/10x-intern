mod common;

fn no_gh_runner() -> common::SequencedCommandRunner {
    common::SequencedCommandRunner::new().then_err("command not found: gh")
}

#[test]
fn detect_wizard_hints_populates_repo_when_gh_available() {
    let runner = common::SequencedCommandRunner::new()
        .then_ok("2.0.0\n")        // gh --version
        .then_ok("acme/widgets\n"); // gh repo view

    let dir = tempfile::tempdir().unwrap();
    let hints = intern::behaviors::detect_wizard_hints(dir.path(), &runner);
    assert_eq!(hints.repo, Some("acme/widgets".to_string()));
}

#[test]
fn detect_wizard_hints_repo_is_none_when_gh_not_available() {
    let runner = common::SequencedCommandRunner::new()
        .then_err("command not found: gh");

    let dir = tempfile::tempdir().unwrap();
    let hints = intern::behaviors::detect_wizard_hints(dir.path(), &runner);
    assert_eq!(hints.repo, None);
}

#[test]
fn detect_wizard_hints_finds_claude_md_as_context_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("CLAUDE.md"), "").unwrap();
    let hints = intern::behaviors::detect_wizard_hints(dir.path(), &no_gh_runner());
    assert_eq!(hints.context_file, Some("CLAUDE.md".to_string()));
}

#[test]
fn detect_wizard_hints_finds_agents_md_as_context_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("AGENTS.md"), "").unwrap();
    let hints = intern::behaviors::detect_wizard_hints(dir.path(), &no_gh_runner());
    assert_eq!(hints.context_file, Some("AGENTS.md".to_string()));
}

#[test]
fn detect_wizard_hints_context_file_is_none_when_no_known_files() {
    let dir = tempfile::tempdir().unwrap();
    let hints = intern::behaviors::detect_wizard_hints(dir.path(), &no_gh_runner());
    assert_eq!(hints.context_file, None);
}

#[test]
fn detect_wizard_hints_detects_git_source_control_when_dot_git_exists() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    let hints = intern::behaviors::detect_wizard_hints(dir.path(), &no_gh_runner());
    assert_eq!(hints.source_control_kind, Some(intern::traits::SourceControlKind::Git));
}

#[test]
fn detect_wizard_hints_source_control_is_none_when_no_dot_git() {
    let dir = tempfile::tempdir().unwrap();
    let hints = intern::behaviors::detect_wizard_hints(dir.path(), &no_gh_runner());
    assert_eq!(hints.source_control_kind, None);
}

#[test]
fn wizard_uses_hint_repo_as_default() {
    use intern::behaviors::{WizardHints, interactive_config_wizard};
    use intern::traits::SourceControlKind;

    let hints = WizardHints {
        repo: Some("detected/repo".to_string()),
        context_file: None,
        source_control_kind: None,
    };
    // No text queued — wizard should fall back to the hint default
    let interactor = common::StubInteractor::new()
        .with_choice(0)     // issue tracker: github
        // repo: no text queued, hint default used
        .with_choice(0)     // source control: git
        .with_choice(0)     // agent: local
        .with_confirm(false) // settings file: no
        .with_confirm(false) // context file: no
        .with_choice(0)     // commit strategy: direct
        .with_confirm(true); // summary

    let dir = tempfile::tempdir().unwrap();
    let output = interactive_config_wizard(dir.path(), &interactor, &hints).unwrap();
    assert_eq!(output.repo, "detected/repo");
}

#[test]
fn wizard_uses_hint_source_control_kind_as_default() {
    use intern::behaviors::{WizardHints, interactive_config_wizard};
    use intern::traits::SourceControlKind;

    // Verify the wizard correctly computes the default index for source_control
    // from the hint and passes it through. We queue the same index the hint would
    // pre-select (1 = None) to simulate a user accepting the pre-selected default.
    let hints = WizardHints {
        repo: None,
        context_file: None,
        source_control_kind: Some(SourceControlKind::None),
    };
    let interactor = common::StubInteractor::new()
        .with_choice(0)          // issue tracker: github
        .with_text("owner/repo") // repo
        .with_choice(1)          // source control: None (index 1 — matches hinted default)
        .with_choice(0)          // agent: local
        .with_confirm(false)     // settings file: no
        .with_confirm(false)     // context file: no
        .with_choice(0)          // commit strategy: direct
        .with_confirm(true);     // summary

    let dir = tempfile::tempdir().unwrap();
    let output = interactive_config_wizard(dir.path(), &interactor, &hints).unwrap();
    assert_eq!(output.source_control_kind, SourceControlKind::None);
}

#[test]
fn wizard_uses_hint_context_file_as_default_when_user_confirms() {
    use intern::behaviors::{WizardHints, interactive_config_wizard};

    let hints = WizardHints {
        repo: None,
        context_file: Some("CLAUDE.md".to_string()),
        source_control_kind: None,
    };
    let interactor = common::StubInteractor::new()
        .with_choice(0)          // issue tracker: github
        .with_text("owner/repo") // repo
        .with_choice(0)          // source control: git
        .with_choice(0)          // agent: local
        .with_confirm(false)     // settings file: no
        .with_confirm(true)      // context file: yes
        // no text queued for context file path — hint default used
        .with_choice(0)          // commit strategy: direct
        .with_confirm(true);     // summary

    let dir = tempfile::tempdir().unwrap();
    let output = interactive_config_wizard(dir.path(), &interactor, &hints).unwrap();
    assert_eq!(output.context_file, Some("CLAUDE.md".to_string()));
}
