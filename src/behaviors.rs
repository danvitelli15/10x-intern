use anyhow::Result;

use crate::actions::{
    create_file, detect_repo_slug, feature_review, find_file, generate_test_instructions,
    implement, plan_order, review,
};
use crate::context::{BudgetExhausted, Context};
use crate::traits::{
    AgentKind, CommandRunner, IssueTrackerKind, IssueType, MergeStrategy, SourceControlKind,
};

pub struct WizardHints {
    pub repo: Option<String>,
    pub context_file: Option<String>,
    pub source_control_kind: Option<SourceControlKind>,
}

impl WizardHints {
    pub fn none() -> Self {
        Self {
            repo: None,
            context_file: None,
            source_control_kind: None,
        }
    }
}

const KNOWN_CONTEXT_FILES: &[&str] = &["CLAUDE.md", "AGENTS.md"];

pub fn detect_wizard_hints(base_dir: &std::path::Path, runner: &dyn CommandRunner) -> WizardHints {
    log::debug!("detect_wizard_hints: scanning {}", base_dir.display());
    let repo = detect_repo_slug(runner);
    log::trace!("detect_wizard_hints: repo slug — {:?}", repo);
    let context_file = KNOWN_CONTEXT_FILES
        .iter()
        .find(|&&name| find_file(base_dir, name).is_some())
        .map(|&name| name.to_string());
    log::trace!("detect_wizard_hints: context file — {:?}", context_file);
    let source_control_kind = if find_file(base_dir, ".git").is_some() {
        log::trace!("detect_wizard_hints: .git directory found");
        Some(SourceControlKind::Git)
    } else {
        log::trace!("detect_wizard_hints: no .git directory found");
        None
    };
    WizardHints {
        repo,
        context_file,
        source_control_kind,
    }
}

pub trait UserInteractor {
    fn prompt_text(&self, question: &str, default: Option<&str>) -> Result<String>;
    fn prompt_choice(
        &self,
        question: &str,
        choices: &[String],
        default_idx: Option<usize>,
    ) -> Result<usize>;
    fn prompt_confirm(&self, question: &str, default: bool) -> Result<bool>;
}

pub struct TerminalInteractor;

impl UserInteractor for TerminalInteractor {
    fn prompt_text(&self, question: &str, default: Option<&str>) -> Result<String> {
        let mut prompt = inquire::Text::new(question);
        let default_owned;
        if let Some(d) = default {
            default_owned = d.to_string();
            prompt = prompt.with_default(&default_owned);
        }
        Ok(prompt.prompt()?)
    }

    fn prompt_choice(
        &self,
        question: &str,
        choices: &[String],
        default_idx: Option<usize>,
    ) -> Result<usize> {
        let mut select = inquire::Select::new(question, choices.to_vec());
        if let Some(idx) = default_idx {
            select = select.with_starting_cursor(idx);
        }
        let answer = select.prompt()?;
        Ok(choices.iter().position(|c| c == &answer).unwrap())
    }

    fn prompt_confirm(&self, question: &str, default: bool) -> Result<bool> {
        Ok(inquire::Confirm::new(question)
            .with_default(default)
            .prompt()?)
    }
}

pub struct WizardOutput {
    pub issue_tracker_kind: IssueTrackerKind,
    pub repo: String,
    pub source_control_kind: SourceControlKind,
    pub agent_kind: AgentKind,
    pub settings_file: Option<String>,
    pub context_file: Option<String>,
    pub merge_strategy: MergeStrategy,
}

impl WizardOutput {
    pub fn defaults() -> Self {
        Self {
            issue_tracker_kind: IssueTrackerKind::GitHub,
            repo: "owner/repo".to_string(),
            source_control_kind: SourceControlKind::Git,
            agent_kind: AgentKind::Local,
            settings_file: None,
            context_file: None,
            merge_strategy: MergeStrategy::FeatureBranch,
        }
    }
}

fn choice_list<T: std::fmt::Display + Copy>(items: &[T]) -> Vec<String>
where
    T: HasDescription,
{
    items
        .iter()
        .map(|i| format!("{} — {}", i.label(), i.description()))
        .collect()
}

trait HasDescription {
    fn label(&self) -> &'static str;
    fn description(&self) -> &'static str;
}

impl HasDescription for IssueTrackerKind {
    fn label(&self) -> &'static str {
        IssueTrackerKind::label(self)
    }
    fn description(&self) -> &'static str {
        IssueTrackerKind::description(self)
    }
}

impl HasDescription for AgentKind {
    fn label(&self) -> &'static str {
        AgentKind::label(self)
    }
    fn description(&self) -> &'static str {
        AgentKind::description(self)
    }
}

impl HasDescription for SourceControlKind {
    fn label(&self) -> &'static str {
        SourceControlKind::label(self)
    }
    fn description(&self) -> &'static str {
        SourceControlKind::description(self)
    }
}

impl HasDescription for MergeStrategy {
    fn label(&self) -> &'static str {
        MergeStrategy::label(self)
    }
    fn description(&self) -> &'static str {
        MergeStrategy::description(self)
    }
}

pub fn interactive_config_wizard(
    _base_dir: &std::path::Path,
    interactor: &dyn UserInteractor,
    hints: &WizardHints,
) -> Result<WizardOutput> {
    let tracker_idx =
        interactor.prompt_choice("Issue tracker", &choice_list(IssueTrackerKind::all()), None)?;
    let issue_tracker_kind = IssueTrackerKind::all()[tracker_idx];

    let repo_default = hints.repo.as_deref().unwrap_or("owner/repo");
    let repo = interactor.prompt_text("Repository (owner/repo)", Some(repo_default))?;

    let sc_default = hints
        .source_control_kind
        .and_then(|k| SourceControlKind::all().iter().position(|v| v == &k));
    let sc_idx = interactor.prompt_choice(
        "Source control",
        &choice_list(SourceControlKind::all()),
        sc_default,
    )?;
    let source_control_kind = SourceControlKind::all()[sc_idx];

    let agent_idx = interactor.prompt_choice("Agent", &choice_list(AgentKind::all()), None)?;
    let agent_kind = AgentKind::all()[agent_idx];

    let settings_file = if interactor.prompt_confirm("Specify an agent settings file?", false)? {
        Some(interactor.prompt_text("Settings file path", Some(".claude/settings.json"))?)
    } else {
        None
    };

    let context_file_default = hints.context_file.as_deref().unwrap_or("CLAUDE.md");
    let context_file =
        if interactor.prompt_confirm("Specify a context file (e.g. CLAUDE.md)?", false)? {
            Some(interactor.prompt_text("Context file path", Some(context_file_default))?)
        } else {
            None
        };

    let strategy_idx =
        interactor.prompt_choice("Merge strategy", &choice_list(MergeStrategy::all()), None)?;
    let merge_strategy = MergeStrategy::all()[strategy_idx];

    let output = WizardOutput {
        issue_tracker_kind,
        repo,
        source_control_kind,
        agent_kind,
        settings_file,
        context_file,
        merge_strategy,
    };

    interactor.prompt_confirm(
        &format!(
            "Settings look good?\n  issue_tracker: {} | repo: {} | agent: {} | merge_strategy: {}",
            output.issue_tracker_kind.key(),
            output.repo,
            output.agent_kind.key(),
            output.merge_strategy.key()
        ),
        true,
    )?;

    Ok(output)
}

const PROMPT_IMPLEMENT: &str = include_str!("../scaffold/prompts/implement.md");
const PROMPT_REVIEW: &str = include_str!("../scaffold/prompts/review.md");
const PROMPT_FEATURE_REVIEW: &str = include_str!("../scaffold/prompts/feature_review.md");
const PROMPT_PLAN_ORDER: &str = include_str!("../scaffold/prompts/plan_order.md");
const PROMPT_TEST_INSTRUCTIONS: &str = include_str!("../scaffold/prompts/test_instructions.md");

pub fn scaffold_intern_directory(base_dir: &std::path::Path, wizard: &WizardOutput) -> Result<()> {
    log::info!("scaffolding .intern directory in {}", base_dir.display());
    let intern_dir = base_dir.join(".intern");
    let prompts_dir = intern_dir.join("prompts");
    log::debug!("scaffold: writing config.toml");
    create_file(
        &intern_dir.join("config.toml"),
        &generate_config_toml(wizard),
    )?;
    log::debug!("scaffold: writing prompt templates");
    create_file(&prompts_dir.join("implement.md"), PROMPT_IMPLEMENT)?;
    create_file(&prompts_dir.join("review.md"), PROMPT_REVIEW)?;
    create_file(
        &prompts_dir.join("feature_review.md"),
        PROMPT_FEATURE_REVIEW,
    )?;
    create_file(&prompts_dir.join("plan_order.md"), PROMPT_PLAN_ORDER)?;
    create_file(
        &prompts_dir.join("test_instructions.md"),
        PROMPT_TEST_INSTRUCTIONS,
    )?;
    log::info!("scaffold complete — run 'intern implement <issue-id>' to start");
    Ok(())
}

fn generate_config_toml(wizard: &WizardOutput) -> String {
    let mut lines = vec![];

    if let Some(ref ctx) = wizard.context_file {
        lines.push(format!("context_file = {:?}", ctx));
    }

    lines.push(String::new());
    lines.push("[issue_tracker]".to_string());
    lines.push(format!("kind = {:?}", wizard.issue_tracker_kind.key()));
    lines.push(format!("repo = {:?}", wizard.repo));

    lines.push(String::new());
    lines.push("[source_control]".to_string());
    lines.push(format!("kind = {:?}", wizard.source_control_kind.key()));
    lines.push("base_branch = \"main\"".to_string());
    lines.push(format!("merge_strategy = {:?}", wizard.merge_strategy.key()));
    lines.push("use_worktree = false".to_string());
    lines.push("on_dirty_after_commit = \"warn\"".to_string());
    lines.push("on_dirty_no_commits = \"fail\"".to_string());

    lines.push(String::new());
    lines.push("[agent]".to_string());
    lines.push(format!("kind = {:?}", wizard.agent_kind.key()));
    if let Some(ref sf) = wizard.settings_file {
        lines.push(format!("settings_file = {:?}", sf));
    }

    lines.push(String::new());
    lines.push("[run]".to_string());
    lines.push("max_iterations = 100".to_string());

    lines.push(String::new());
    lines.join("\n")
}

fn setup_workspace(issue_id: u64, base_branch: &str, ctx: &Context) -> Result<Option<String>> {
    // TODO: when use_worktree is true, provision a git worktree here
    match ctx.config.merge_strategy {
        MergeStrategy::Direct => Ok(None),
        MergeStrategy::PerTicket | MergeStrategy::FeatureBranch => {
            let branch = format!("feature/ticket-{issue_id}");
            ctx.source_control.create_branch(&branch, base_branch)?;
            Ok(Some(branch))
        }
    }
}

pub fn complete_ticket(issue_id: u64, ctx: &Context, base_branch: &str) -> Result<()> {
    log::debug!("complete_ticket: starting implement+review loop for issue #{issue_id}");
    let expected_branch = setup_workspace(issue_id, base_branch, ctx)?;

    loop {
        log::trace!("complete_ticket: beginning iteration for issue #{issue_id}");
        let result = (|| -> Result<bool> {
            implement(issue_id, ctx)?;
            review(issue_id, ctx)
        })();

        match result {
            Ok(false) => {
                log::debug!("complete_ticket: review clean for issue #{issue_id}");
                break;
            }
            Ok(true) => {
                log::info!("review found issues with #{issue_id} — retrying");
                continue;
            }
            Err(e) if e.downcast_ref::<BudgetExhausted>().is_some() => {
                log::info!("budget exhausted for issue #{issue_id} — skipping (hitl)");
                ctx.issues.skip_issue(issue_id)?;
                return Ok(());
            }
            Err(e) => return Err(e),
        }
    }

    if let Some(expected) = expected_branch {
        let actual = ctx.source_control.current_branch()?;
        if actual != expected {
            anyhow::bail!(
                "branch mismatch after implement: expected '{expected}', got '{actual}'"
            );
        }
    }

    log::debug!("complete_ticket: generating test instructions for issue #{issue_id}");
    generate_test_instructions(issue_id, ctx)?;
    Ok(())
}

pub fn complete_feature(issue_id: u64, ctx: &Context, base_branch: &str) -> Result<()> {
    log::debug!("complete_feature: starting for issue #{issue_id}");

    // For FeatureBranch strategy, create a feature-level branch that child tickets branch from.
    // For other strategies, children branch from whatever base was passed in.
    let child_base = match ctx.config.merge_strategy {
        MergeStrategy::FeatureBranch => {
            let feature_branch = format!("feature/ticket-{issue_id}");
            // TODO: when use_worktree is true, provision a git worktree here
            ctx.source_control.create_branch(&feature_branch, base_branch)?;
            feature_branch
        }
        _ => base_branch.to_string(),
    };

    let initial_children = ctx.issues.get_children(issue_id)?;
    log::debug!("complete_feature: {} initial child issue(s) for #{issue_id}", initial_children.len());
    let initial_ids: std::collections::HashSet<u64> =
        initial_children.iter().map(|i| i.id).collect();
    execute_ordered(&initial_children, ctx, &child_base)?;

    log::debug!("complete_feature: running feature review for #{issue_id}");
    let has_findings = feature_review(issue_id, ctx)?;
    if has_findings {
        log::info!("feature review found issues for #{issue_id} — checking for new child issues");
        let all_children = ctx.issues.get_children(issue_id)?;
        let new_children: Vec<_> = all_children
            .into_iter()
            .filter(|i| !initial_ids.contains(&i.id))
            .collect();
        log::debug!("complete_feature: {} new child issue(s) to process", new_children.len());
        execute_ordered(&new_children, ctx, &child_base)?;

        if feature_review(issue_id, ctx)? {
            log::info!("second feature review still has findings for #{issue_id} — skipping (hitl)");
            ctx.issues.skip_issue(issue_id)?;
            return Ok(());
        }
    }

    log::debug!("complete_feature: generating test instructions for #{issue_id}");
    generate_test_instructions(issue_id, ctx)?;
    Ok(())
}

pub fn execute_ordered(issues: &[crate::traits::Issue], ctx: &Context, base_branch: &str) -> Result<()> {
    log::info!("planning execution order for {} issue(s)", issues.len());
    let ordered_ids = plan_order(issues, ctx)?;
    log::debug!("execute_ordered: order — {:?}", ordered_ids);
    for id in &ordered_ids {
        let issue_type = ctx.issues.issue_type(*id)?;
        log::trace!("execute_ordered: issue #{id} is {}", match issue_type { IssueType::Ticket => "Ticket", IssueType::Feature => "Feature" });
        match issue_type {
            IssueType::Ticket => complete_ticket(*id, ctx, base_branch)?,
            IssueType::Feature => complete_feature(*id, ctx, base_branch)?,
        }
    }
    Ok(())
}
