use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use crate::orchestrator::Context;
use crate::traits::Issue;

pub fn create_file(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        anyhow::bail!("file already exists: {}", path.display());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

fn load_prompt(base_dir: &Path, name: &str) -> Result<String> {
    let path = base_dir.join(".intern/prompts").join(format!("{name}.md"));
    if !path.exists() {
        anyhow::bail!(
            "missing prompt file: {} — run 'intern init' to scaffold defaults",
            path.display()
        );
    }
    let raw = std::fs::read_to_string(&path)?;
    Ok(strip_prompt_docs(&raw))
}

fn strip_prompt_docs(s: &str) -> String {
    let mut result = s.to_string();
    while let (Some(start), Some(end)) = (
        result.find("<strip-before-prompting>"),
        result.find("</strip-before-prompting>"),
    ) {
        let end_tag = "</strip-before-prompting>".len();
        result = format!("{}{}", &result[..start], &result[end + end_tag..]);
    }
    result.trim_start().to_string()
}

#[derive(Deserialize)]
struct OrderedItem {
    id: u64,
}

pub fn plan_order(issues: &[Issue], ctx: &Context) -> Result<Vec<u64>> {
    let prompt = build_plan_order_prompt(issues, &ctx.config.work_directory)?;
    let output = ctx.run_agent(&prompt)?;
    let items: Vec<OrderedItem> = serde_json::from_str(output.stdout.trim())?;
    Ok(items.into_iter().map(|item| item.id).collect())
}

pub fn implement(issue_id: u64, ctx: &Context) -> Result<()> {
    let issue = ctx.issues.get_issue(issue_id)?;

    if issue.labels.contains(&"hitl".to_string()) {
        log::info!("skipping issue #{issue_id} — labeled hitl");
        return Ok(());
    }

    ctx.issues.claim_issue(issue_id)?;
    ctx.events.emit(crate::traits::Event::AgentStarted(issue_id));

    let prompt = build_implement_prompt(&issue, &ctx.config.repo_context, &ctx.config.work_directory)?;
    let output = ctx.run_agent(&prompt)?;

    if output.success {
        ctx.issues.complete_issue(issue_id)?;
        ctx.events.emit(crate::traits::Event::IssueComplete(issue_id));
    }

    ctx.events.emit(crate::traits::Event::RunComplete);
    Ok(())
}

pub fn review(issue_id: u64, ctx: &Context) -> Result<bool> {
    let issue = ctx.issues.get_issue(issue_id)?;
    let diff = ctx.source_control.diff_from_base("main")?;
    let prompt = build_review_prompt(&issue, &diff, &ctx.config.work_directory)?;
    let output = ctx.run_agent(&prompt)?;
    Ok(output.stdout.contains("<reviewResult>FINDINGS</reviewResult>"))
}

pub fn feature_review(issue_id: u64, ctx: &Context) -> Result<bool> {
    let issue = ctx.issues.get_issue(issue_id)?;
    let diff = ctx.source_control.diff_from_base("main")?;
    let prompt = build_feature_review_prompt(&issue, &diff, &ctx.config.work_directory)?;
    let output = ctx.run_agent(&prompt)?;
    Ok(output.stdout.contains("<featureReviewResult>IN_SCOPE_FINDINGS</featureReviewResult>"))
}

pub fn generate_test_instructions(issue_id: u64, ctx: &Context) -> Result<()> {
    let issue = ctx.issues.get_issue(issue_id)?;
    let diff = ctx.source_control.diff_from_base("main")?;
    let prompt = build_test_instructions_prompt(&issue, &diff, &ctx.config.work_directory)?;
    ctx.run_agent(&prompt)?;
    Ok(())
}

fn build_implement_prompt(issue: &Issue, repo_context: &str, work_directory: &Path) -> Result<String> {
    let template = load_prompt(work_directory, "implement")?;
    Ok(template
        .replace("{{issue_id}}", &issue.id.to_string())
        .replace("{{issue_title}}", &issue.title)
        .replace("{{issue_body}}", &issue.body)
        .replace("{{repo_context}}", repo_context))
}

fn build_plan_order_prompt(issues: &[Issue], work_directory: &Path) -> Result<String> {
    let template = load_prompt(work_directory, "plan_order")?;
    let issues_list = issues.iter()
        .map(|i| format!("### Issue #{}: {}\n{}", i.id, i.title, i.body))
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok(template.replace("{{issues_list}}", &issues_list))
}

fn build_feature_review_prompt(issue: &Issue, diff: &str, work_directory: &Path) -> Result<String> {
    let template = load_prompt(work_directory, "feature_review")?;
    Ok(template
        .replace("{{issue_id}}", &issue.id.to_string())
        .replace("{{issue_title}}", &issue.title)
        .replace("{{issue_body}}", &issue.body)
        .replace("{{diff}}", diff))
}

fn build_review_prompt(issue: &Issue, diff: &str, work_directory: &Path) -> Result<String> {
    let template = load_prompt(work_directory, "review")?;
    Ok(template
        .replace("{{issue_id}}", &issue.id.to_string())
        .replace("{{issue_title}}", &issue.title)
        .replace("{{issue_body}}", &issue.body)
        .replace("{{diff}}", diff))
}

fn build_test_instructions_prompt(issue: &Issue, diff: &str, work_directory: &Path) -> Result<String> {
    let template = load_prompt(work_directory, "test_instructions")?;
    Ok(template
        .replace("{{issue_id}}", &issue.id.to_string())
        .replace("{{issue_title}}", &issue.title)
        .replace("{{issue_body}}", &issue.body)
        .replace("{{diff}}", diff))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::Context;
    use crate::traits::{
        AgentOutput, AgentRunner, CommitStrategy, Event, EventSink, Issue, IssueTracker, IssueType,
        RemoteClient, RunConfig, SourceControl,
    };

    struct StubIssueTracker;
    impl IssueTracker for StubIssueTracker {
        fn get_issue(&self, id: u64) -> anyhow::Result<Issue> {
            Ok(Issue { id, title: "T".into(), body: "B".into(), labels: vec![] })
        }
        fn get_children(&self, _: u64) -> anyhow::Result<Vec<Issue>> { Ok(vec![]) }
        fn get_issues_by_label(&self, _: &str) -> anyhow::Result<Vec<Issue>> { Ok(vec![]) }
        fn claim_issue(&self, _: u64) -> anyhow::Result<()> { Ok(()) }
        fn complete_issue(&self, _: u64) -> anyhow::Result<()> { Ok(()) }
        fn skip_issue(&self, _: u64) -> anyhow::Result<()> { Ok(()) }
        fn post_comment(&self, _: u64, _: &str) -> anyhow::Result<()> { Ok(()) }
        fn create_child_issue(&self, _: u64, _: &str, _: &str) -> anyhow::Result<Issue> { unimplemented!() }
        fn issue_type(&self, _: u64) -> anyhow::Result<IssueType> { Ok(IssueType::Ticket) }
    }

    struct StubSourceControl;
    impl SourceControl for StubSourceControl {
        fn create_branch(&self, _: &str) -> anyhow::Result<()> { Ok(()) }
        fn current_branch(&self) -> anyhow::Result<String> { Ok("main".into()) }
        fn diff_from_base(&self, _: &str) -> anyhow::Result<String> { Ok("".into()) }
        fn stage(&self, _: Option<&[&str]>) -> anyhow::Result<()> { Ok(()) }
        fn commit(&self, _: &str) -> anyhow::Result<()> { Ok(()) }
    }

    struct StubRemoteClient;
    impl RemoteClient for StubRemoteClient {
        fn create_pr(&self, _: &str, _: &str, _: &str) -> anyhow::Result<String> { Ok("".into()) }
    }

    struct StubEventSink;
    impl EventSink for StubEventSink {
        fn emit(&self, _: Event) {}
    }

    struct FixedRunner { stdout: String }
    impl AgentRunner for FixedRunner {
        fn run(&self, _: &str, _: &RunConfig) -> anyhow::Result<AgentOutput> {
            Ok(AgentOutput { stdout: self.stdout.clone(), success: true })
        }
    }

    fn make_all_prompts_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let prompts_dir = dir.path().join(".intern/prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        for name in &["implement", "review", "feature_review", "plan_order", "test_instructions"] {
            std::fs::write(prompts_dir.join(format!("{name}.md")), "{{issue_id}} {{issue_title}} {{issue_body}} {{diff}} {{issues_list}} {{repo_context}}").unwrap();
        }
        dir
    }

    fn test_context(stdout: &str) -> (Context, tempfile::TempDir) {
        let dir = make_all_prompts_dir();
        let ctx = Context::new(
            Box::new(StubIssueTracker),
            Box::new(StubSourceControl),
            Box::new(StubRemoteClient),
            Box::new(FixedRunner { stdout: stdout.to_string() }),
            Box::new(StubEventSink),
            RunConfig { max_iterations: 10, commit_strategy: CommitStrategy::Direct, dry_run: false, repo_context: String::new(), work_directory: dir.path().to_path_buf() },
        );
        (ctx, dir)
    }

    #[test]
    fn feature_review_returns_true_when_agent_outputs_in_scope_findings() {
        let (ctx, _dir) = test_context("Analysis...\n<featureReviewResult>IN_SCOPE_FINDINGS</featureReviewResult>");
        assert!(feature_review(1, &ctx).unwrap());
    }

    #[test]
    fn feature_review_returns_false_when_agent_outputs_clean() {
        let (ctx, _dir) = test_context("Looks good.\n<featureReviewResult>CLEAN</featureReviewResult>");
        assert!(!feature_review(1, &ctx).unwrap());
    }

    #[test]
    fn feature_review_does_not_false_positive_on_untagged_in_scope_findings() {
        let (ctx, _dir) = test_context("I found IN_SCOPE_FINDINGS in the codebase.");
        assert!(!feature_review(1, &ctx).unwrap());
    }

    #[test]
    fn plan_order_parses_agent_json_into_ordered_ids() {
        let issues = vec![
            Issue { id: 1, title: "First".into(), body: "".into(), labels: vec![] },
            Issue { id: 2, title: "Second".into(), body: "".into(), labels: vec![] },
        ];
        let (ctx, _dir) = test_context(r#"[{"id": 2}, {"id": 1}]"#);
        let order = plan_order(&issues, &ctx).unwrap();
        assert_eq!(order, vec![2, 1]);
    }

    #[test]
    fn plan_order_returns_error_for_invalid_json() {
        let issues = vec![Issue { id: 1, title: "T".into(), body: "".into(), labels: vec![] }];
        let (ctx, _dir) = test_context("I think issue 2 should go first, then issue 1.");
        assert!(plan_order(&issues, &ctx).is_err());
    }

    #[test]
    fn review_returns_true_when_agent_outputs_findings() {
        let (ctx, _dir) = test_context("Some analysis...\n<reviewResult>FINDINGS</reviewResult>");
        assert!(review(1, &ctx).unwrap());
    }

    #[test]
    fn review_returns_false_when_agent_outputs_clean() {
        let (ctx, _dir) = test_context("Looks good.\n<reviewResult>CLEAN</reviewResult>");
        assert!(!review(1, &ctx).unwrap());
    }

    #[test]
    fn review_does_not_false_positive_on_untagged_findings() {
        let (ctx, _dir) = test_context("I found several FINDINGS in the analysis.");
        assert!(!review(1, &ctx).unwrap());
    }

    #[test]
    fn load_prompt_uses_override_file_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let prompts_dir = dir.path().join(".intern/prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        std::fs::write(prompts_dir.join("implement.md"), "custom prompt").unwrap();
        let result = load_prompt(dir.path(), "implement").unwrap();
        assert_eq!(result, "custom prompt");
    }

    #[test]
    fn load_prompt_errors_when_file_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_prompt(dir.path(), "implement");
        assert!(result.is_err());
    }

    #[test]
    fn load_prompt_strips_strip_before_prompting_sections() {
        let dir = tempfile::tempdir().unwrap();
        let prompts_dir = dir.path().join(".intern/prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        std::fs::write(prompts_dir.join("implement.md"), "\
<strip-before-prompting>
# Available variables
# {{issue_id}} — the issue number
</strip-before-prompting>

The actual prompt content.").unwrap();
        let result = load_prompt(dir.path(), "implement").unwrap();
        assert!(!result.contains("<strip-before-prompting>"));
        assert!(!result.contains("Available variables"));
        assert!(result.contains("The actual prompt content."));
    }

    fn make_prompt_dir_with(name: &str, content: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let prompts_dir = dir.path().join(".intern/prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        std::fs::write(prompts_dir.join(format!("{name}.md")), content).unwrap();
        dir
    }

    #[test]
    fn implement_prompt_includes_repo_context() {
        let dir = make_prompt_dir_with("implement", "{{repo_context}}\n{{issue_id}}");
        let issue = Issue { id: 1, title: "T".into(), body: "B".into(), labels: vec![] };
        let prompt = build_implement_prompt(&issue, "use snake_case everywhere", dir.path()).unwrap();
        assert!(prompt.contains("use snake_case everywhere"));
    }

    #[test]
    fn implement_prompt_with_empty_repo_context_does_not_panic() {
        let dir = make_prompt_dir_with("implement", "{{repo_context}}\n{{issue_id}}");
        let issue = Issue { id: 1, title: "T".into(), body: "B".into(), labels: vec![] };
        let prompt = build_implement_prompt(&issue, "", dir.path()).unwrap();
        assert!(!prompt.is_empty());
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
    fn create_file_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/c/output.txt");
        create_file(&path, "hello").unwrap();
        assert!(path.exists());
    }
}
