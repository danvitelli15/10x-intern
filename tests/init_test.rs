use intern::workflows::init_workflow;

#[test]
fn init_workflow_creates_config_file() {
    let dir = tempfile::tempdir().unwrap();
    init_workflow(dir.path()).unwrap();
    assert!(dir.path().join(".intern/config.toml").exists());
}

#[test]
fn init_workflow_creates_all_prompt_files() {
    let dir = tempfile::tempdir().unwrap();
    init_workflow(dir.path()).unwrap();
    for name in &["implement", "review", "feature_review", "plan_order", "test_instructions"] {
        assert!(dir.path().join(format!(".intern/prompts/{name}.md")).exists(), "{name}.md missing");
    }
}

#[test]
fn init_workflow_errors_if_already_initialized() {
    let dir = tempfile::tempdir().unwrap();
    init_workflow(dir.path()).unwrap();
    assert!(init_workflow(dir.path()).is_err());
}
