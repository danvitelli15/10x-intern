use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use crate::orchestrator::Context;
use crate::traits::Issue;

fn load_prompt(base_dir: &Path, name: &str, builtin: &str) -> String {
    let override_path = base_dir.join(".intern/prompts").join(format!("{name}.md"));
    if override_path.exists() {
        std::fs::read_to_string(&override_path).unwrap_or_else(|_| builtin.to_string())
    } else {
        builtin.to_string()
    }
}

#[derive(Deserialize)]
struct OrderedItem {
    id: u64,
}

pub fn plan_order(issues: &[Issue], ctx: &Context) -> Result<Vec<u64>> {
    let prompt = build_plan_order_prompt(issues, &ctx.config.work_directory);
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

    let prompt = build_implement_prompt(&issue, &ctx.config.repo_context, &ctx.config.work_directory);
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
    let prompt = build_review_prompt(&issue, &diff, &ctx.config.work_directory);
    let output = ctx.run_agent(&prompt)?;
    Ok(output.stdout.contains("<reviewResult>FINDINGS</reviewResult>"))
}

pub fn feature_review(issue_id: u64, ctx: &Context) -> Result<bool> {
    let issue = ctx.issues.get_issue(issue_id)?;
    let diff = ctx.source_control.diff_from_base("main")?;
    let prompt = build_feature_review_prompt(&issue, &diff, &ctx.config.work_directory);
    let output = ctx.run_agent(&prompt)?;
    Ok(output.stdout.contains("<featureReviewResult>IN_SCOPE_FINDINGS</featureReviewResult>"))
}

pub fn generate_test_instructions(issue_id: u64, ctx: &Context) -> Result<()> {
    let issue = ctx.issues.get_issue(issue_id)?;
    let diff = ctx.source_control.diff_from_base("main")?;
    let prompt = build_test_instructions_prompt(&issue, &diff, &ctx.config.work_directory);
    ctx.run_agent(&prompt)?;
    Ok(())
}

fn build_implement_prompt(issue: &Issue, repo_context: &str, work_directory: &Path) -> String {
    let template = load_prompt(work_directory, "implement", include_str!("../prompts/implement.md"));
    template
        .replace("{{issue_id}}", &issue.id.to_string())
        .replace("{{issue_title}}", &issue.title)
        .replace("{{issue_body}}", &issue.body)
        .replace("{{repo_context}}", repo_context)
}

fn build_plan_order_prompt(issues: &[Issue], work_directory: &Path) -> String {
    let template = load_prompt(work_directory, "plan_order", include_str!("../prompts/plan_order.md"));
    let issues_list = issues.iter()
        .map(|i| format!("### Issue #{}: {}\n{}", i.id, i.title, i.body))
        .collect::<Vec<_>>()
        .join("\n\n");
    template.replace("{{issues_list}}", &issues_list)
}

fn build_feature_review_prompt(issue: &Issue, diff: &str, work_directory: &Path) -> String {
    let template = load_prompt(work_directory, "feature_review", include_str!("../prompts/feature_review.md"));
    template
        .replace("{{issue_id}}", &issue.id.to_string())
        .replace("{{issue_title}}", &issue.title)
        .replace("{{issue_body}}", &issue.body)
        .replace("{{diff}}", diff)
}

fn build_review_prompt(issue: &Issue, diff: &str, work_directory: &Path) -> String {
    let template = load_prompt(work_directory, "review", include_str!("../prompts/review.md"));
    template
        .replace("{{issue_id}}", &issue.id.to_string())
        .replace("{{issue_title}}", &issue.title)
        .replace("{{issue_body}}", &issue.body)
        .replace("{{diff}}", diff)
}

fn build_test_instructions_prompt(issue: &Issue, diff: &str, work_directory: &Path) -> String {
    let template = load_prompt(work_directory, "test_instructions", include_str!("../prompts/test_instructions.md"));
    template
        .replace("{{issue_id}}", &issue.id.to_string())
        .replace("{{issue_title}}", &issue.title)
        .replace("{{issue_body}}", &issue.body)
        .replace("{{diff}}", diff)
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

    fn test_context(stdout: &str) -> Context {
        Context::new(
            Box::new(StubIssueTracker),
            Box::new(StubSourceControl),
            Box::new(StubRemoteClient),
            Box::new(FixedRunner { stdout: stdout.to_string() }),
            Box::new(StubEventSink),
            RunConfig { max_iterations: 10, commit_strategy: CommitStrategy::Direct, dry_run: false, repo_context: String::new(), work_directory: std::path::PathBuf::from(".") },
        )
    }

    #[test]
    fn feature_review_returns_true_when_agent_outputs_in_scope_findings() {
        let ctx = test_context("Analysis...\n<featureReviewResult>IN_SCOPE_FINDINGS</featureReviewResult>");
        assert!(feature_review(1, &ctx).unwrap());
    }

    #[test]
    fn feature_review_returns_false_when_agent_outputs_clean() {
        let ctx = test_context("Looks good.\n<featureReviewResult>CLEAN</featureReviewResult>");
        assert!(!feature_review(1, &ctx).unwrap());
    }

    #[test]
    fn feature_review_does_not_false_positive_on_untagged_in_scope_findings() {
        let ctx = test_context("I found IN_SCOPE_FINDINGS in the codebase.");
        assert!(!feature_review(1, &ctx).unwrap());
    }

    #[test]
    fn plan_order_parses_agent_json_into_ordered_ids() {
        let issues = vec![
            Issue { id: 1, title: "First".into(), body: "".into(), labels: vec![] },
            Issue { id: 2, title: "Second".into(), body: "".into(), labels: vec![] },
        ];
        let ctx = test_context(r#"[{"id": 2}, {"id": 1}]"#);
        let order = plan_order(&issues, &ctx).unwrap();
        assert_eq!(order, vec![2, 1]);
    }

    #[test]
    fn plan_order_returns_error_for_invalid_json() {
        let issues = vec![Issue { id: 1, title: "T".into(), body: "".into(), labels: vec![] }];
        let ctx = test_context("I think issue 2 should go first, then issue 1.");
        assert!(plan_order(&issues, &ctx).is_err());
    }

    #[test]
    fn review_returns_true_when_agent_outputs_findings() {
        let ctx = test_context("Some analysis...\n<reviewResult>FINDINGS</reviewResult>");
        assert!(review(1, &ctx).unwrap());
    }

    #[test]
    fn review_returns_false_when_agent_outputs_clean() {
        let ctx = test_context("Looks good.\n<reviewResult>CLEAN</reviewResult>");
        assert!(!review(1, &ctx).unwrap());
    }

    #[test]
    fn review_does_not_false_positive_on_untagged_findings() {
        let ctx = test_context("I found several FINDINGS in the analysis.");
        assert!(!review(1, &ctx).unwrap());
    }

    #[test]
    fn load_prompt_uses_override_file_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let prompts_dir = dir.path().join(".intern/prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        std::fs::write(prompts_dir.join("implement.md"), "custom prompt").unwrap();
        let result = load_prompt(dir.path(), "implement", "builtin prompt");
        assert_eq!(result, "custom prompt");
    }

    #[test]
    fn load_prompt_falls_back_to_builtin_when_no_override() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_prompt(dir.path(), "implement", "builtin prompt");
        assert_eq!(result, "builtin prompt");
    }

    #[test]
    fn implement_prompt_includes_repo_context() {
        let issue = Issue { id: 1, title: "T".into(), body: "B".into(), labels: vec![] };
        let prompt = build_implement_prompt(&issue, "use snake_case everywhere", std::path::Path::new("."));
        assert!(prompt.contains("use snake_case everywhere"));
    }

    #[test]
    fn implement_prompt_with_empty_repo_context_does_not_panic() {
        let issue = Issue { id: 1, title: "T".into(), body: "B".into(), labels: vec![] };
        let prompt = build_implement_prompt(&issue, "", std::path::Path::new("."));
        assert!(!prompt.is_empty());
    }
}
