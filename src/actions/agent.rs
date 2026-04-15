use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use crate::context::Context;
use crate::traits::Issue;

pub fn plan_order(issues: &[Issue], ctx: &Context) -> Result<Vec<u64>> {
    log::debug!("plan_order: planning {} issue(s)", issues.len());
    let prompt = build_plan_order_prompt(issues, &ctx.config.work_directory)?;
    let output = ctx.run_agent(&prompt)?;
    let items: Vec<OrderedItem> = serde_json::from_str(output.stdout.trim())?;
    let ids: Vec<u64> = items.into_iter().map(|item| item.id).collect();
    log::debug!("plan_order: execution order — {:?}", ids);
    Ok(ids)
}

#[cfg(test)]
mod plan_order_tests {
    use super::*;
    use super::test_support::test_context;

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
}

pub fn implement(issue_id: u64, ctx: &Context) -> Result<()> {
    log::debug!("implement: fetching issue #{issue_id}");
    let issue = ctx.issues.get_issue(issue_id)?;
    log::trace!("implement: issue #{issue_id} labels — {:?}", issue.labels);

    if issue.labels.contains(&"hitl".to_string()) {
        log::info!("skipping issue #{issue_id} — labeled hitl");
        return Ok(());
    }

    log::info!("claiming issue #{issue_id}");
    ctx.issues.claim_issue(issue_id)?;
    ctx.events.emit(crate::traits::Event::AgentStarted(issue_id));

    log::debug!("implement: building prompt for issue #{issue_id}");
    let prompt = build_implement_prompt(&issue, &ctx.config.repo_context, &ctx.config.work_directory)?;
    log::debug!("implement: running agent for issue #{issue_id}");
    let output = ctx.run_agent(&prompt)?;
    log::trace!("implement: agent returned success={} for issue #{issue_id}", output.success);

    if output.success {
        log::info!("issue #{issue_id} implemented successfully");
        ctx.issues.complete_issue(issue_id)?;
        ctx.events.emit(crate::traits::Event::IssueComplete(issue_id));
    } else {
        log::info!("agent did not succeed for issue #{issue_id}");
    }

    ctx.events.emit(crate::traits::Event::RunComplete);
    Ok(())
}

pub fn review(issue_id: u64, ctx: &Context) -> Result<bool> {
    log::debug!("review: running for issue #{issue_id}");
    let issue = ctx.issues.get_issue(issue_id)?;
    let diff = ctx.source_control.diff_from_base("main")?;
    log::trace!("review: diff is {} bytes", diff.len());
    let prompt = build_review_prompt(&issue, &diff, &ctx.config.work_directory)?;
    let output = ctx.run_agent(&prompt)?;
    let has_findings = output.stdout.contains("<reviewResult>FINDINGS</reviewResult>");
    log::debug!("review: issue #{issue_id} — {}", if has_findings { "FINDINGS" } else { "CLEAN" });
    Ok(has_findings)
}

#[cfg(test)]
mod review_tests {
    use super::*;
    use super::test_support::test_context;

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
}

pub fn feature_review(issue_id: u64, ctx: &Context) -> Result<bool> {
    log::debug!("feature_review: running for issue #{issue_id}");
    let issue = ctx.issues.get_issue(issue_id)?;
    let diff = ctx.source_control.diff_from_base("main")?;
    log::trace!("feature_review: diff is {} bytes", diff.len());
    let prompt = build_feature_review_prompt(&issue, &diff, &ctx.config.work_directory)?;
    let output = ctx.run_agent(&prompt)?;
    let has_findings = output.stdout.contains("<featureReviewResult>IN_SCOPE_FINDINGS</featureReviewResult>");
    log::debug!("feature_review: issue #{issue_id} — {}", if has_findings { "IN_SCOPE_FINDINGS" } else { "CLEAN" });
    Ok(has_findings)
}

#[cfg(test)]
mod feature_review_tests {
    use super::*;
    use super::test_support::test_context;

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
}

pub fn generate_test_instructions(issue_id: u64, ctx: &Context) -> Result<()> {
    log::debug!("generate_test_instructions: running for issue #{issue_id}");
    let issue = ctx.issues.get_issue(issue_id)?;
    let diff = ctx.source_control.diff_from_base("main")?;
    log::trace!("generate_test_instructions: diff is {} bytes", diff.len());
    let prompt = build_test_instructions_prompt(&issue, &diff, &ctx.config.work_directory)?;
    ctx.run_agent(&prompt)?;
    log::debug!("generate_test_instructions: complete for issue #{issue_id}");
    Ok(())
}

#[derive(Deserialize)]
struct OrderedItem {
    id: u64,
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

#[cfg(test)]
mod strip_prompt_docs_tests {
    use super::*;

    #[test]
    fn strip_prompt_docs_removes_tagged_section() {
        let input = "<strip-before-prompting>\n# comment\n</strip-before-prompting>\n\nContent.";
        let result = strip_prompt_docs(input);
        assert!(!result.contains("<strip-before-prompting>"));
        assert!(!result.contains("# comment"));
        assert!(result.contains("Content."));
    }

    #[test]
    fn strip_prompt_docs_handles_multiple_sections() {
        let input = "<strip-before-prompting>\n# first\n</strip-before-prompting>\n\nA.\n\n<strip-before-prompting>\n# second\n</strip-before-prompting>\n\nB.";
        let result = strip_prompt_docs(input);
        assert!(!result.contains("<strip-before-prompting>"));
        assert!(!result.contains("# first"));
        assert!(!result.contains("# second"));
        assert!(result.contains("A."));
        assert!(result.contains("B."));
    }
}

fn load_prompt(base_dir: &Path, name: &str) -> Result<String> {
    let path = base_dir.join(".intern/prompts").join(format!("{name}.md"));
    log::trace!("load_prompt: loading '{name}' from {}", path.display());
    if !path.exists() {
        anyhow::bail!(
            "missing prompt file: {} — run 'intern init' to scaffold defaults",
            path.display()
        );
    }
    let raw = std::fs::read_to_string(&path)?;
    log::trace!("load_prompt: '{name}' loaded ({} chars)", raw.len());
    Ok(strip_prompt_docs(&raw))
}

#[cfg(test)]
mod load_prompt_tests {
    use super::*;
    use super::test_support::make_prompt_dir_with;

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
    fn load_prompt_applies_stripping() {
        let dir = tempfile::tempdir().unwrap();
        let prompts_dir = dir.path().join(".intern/prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        std::fs::write(prompts_dir.join("implement.md"),
            "<strip-before-prompting>\n# docs\n</strip-before-prompting>\n\nContent.").unwrap();
        let result = load_prompt(dir.path(), "implement").unwrap();
        assert!(!result.contains("# docs"));
        assert!(result.contains("Content."));
    }

    #[test]
    fn scaffold_implement_loads_and_strips_cleanly() {
        let dir = make_prompt_dir_with("implement", include_str!("../../scaffold/prompts/implement.md"));
        let result = load_prompt(dir.path(), "implement").unwrap();
        assert!(!result.contains("<strip-before-prompting>"));
        assert!(!result.contains("Available variables"));
        assert!(result.contains("{{issue_id}}"));
    }

    #[test]
    fn scaffold_review_loads_and_strips_cleanly() {
        let dir = make_prompt_dir_with("review", include_str!("../../scaffold/prompts/review.md"));
        let result = load_prompt(dir.path(), "review").unwrap();
        assert!(!result.contains("<strip-before-prompting>"));
        assert!(!result.contains("Available variables"));
        assert!(result.contains("{{diff}}"));
    }

    #[test]
    fn scaffold_feature_review_loads_and_strips_cleanly() {
        let dir = make_prompt_dir_with("feature_review", include_str!("../../scaffold/prompts/feature_review.md"));
        let result = load_prompt(dir.path(), "feature_review").unwrap();
        assert!(!result.contains("<strip-before-prompting>"));
        assert!(!result.contains("Available variables"));
        assert!(result.contains("{{diff}}"));
    }

    #[test]
    fn scaffold_plan_order_loads_and_strips_cleanly() {
        let dir = make_prompt_dir_with("plan_order", include_str!("../../scaffold/prompts/plan_order.md"));
        let result = load_prompt(dir.path(), "plan_order").unwrap();
        assert!(!result.contains("<strip-before-prompting>"));
        assert!(!result.contains("Available variables"));
        assert!(result.contains("{{issues_list}}"));
    }

    #[test]
    fn scaffold_test_instructions_loads_and_strips_cleanly() {
        let dir = make_prompt_dir_with("test_instructions", include_str!("../../scaffold/prompts/test_instructions.md"));
        let result = load_prompt(dir.path(), "test_instructions").unwrap();
        assert!(!result.contains("<strip-before-prompting>"));
        assert!(!result.contains("Available variables"));
        assert!(result.contains("{{issue_id}}"));
    }
}

fn build_implement_prompt(issue: &Issue, repo_context: &str, work_directory: &Path) -> Result<String> {
    let template = load_prompt(work_directory, "implement")?;
    Ok(template
        .replace("{{issue_id}}", &issue.id.to_string())
        .replace("{{issue_title}}", &issue.title)
        .replace("{{issue_body}}", &issue.body)
        .replace("{{repo_context}}", repo_context))
}

#[cfg(test)]
mod build_implement_prompt_tests {
    use super::*;
    use super::test_support::make_prompt_dir_with;

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
mod test_support {
    use crate::context::Context;
    use crate::test_utils::{StubEventSink, StubIssueTracker, StubRemoteClient, StubSourceControl};
    use crate::traits::{AgentOutput, AgentRunner, CommitStrategy, RunConfig};

    pub struct FixedRunner { pub stdout: String }
    impl AgentRunner for FixedRunner {
        fn run(&self, _: &str, _: &RunConfig) -> anyhow::Result<AgentOutput> {
            Ok(AgentOutput { stdout: self.stdout.clone(), success: true })
        }
    }

    pub fn make_all_prompts_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let prompts_dir = dir.path().join(".intern/prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        for name in &["implement", "review", "feature_review", "plan_order", "test_instructions"] {
            std::fs::write(prompts_dir.join(format!("{name}.md")), "{{issue_id}} {{issue_title}} {{issue_body}} {{diff}} {{issues_list}} {{repo_context}}").unwrap();
        }
        dir
    }

    pub fn test_context(stdout: &str) -> (Context, tempfile::TempDir) {
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

    pub fn make_prompt_dir_with(name: &str, content: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let prompts_dir = dir.path().join(".intern/prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        std::fs::write(prompts_dir.join(format!("{name}.md")), content).unwrap();
        dir
    }
}
