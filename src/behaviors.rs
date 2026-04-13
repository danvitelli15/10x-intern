use anyhow::Result;

use crate::actions::{
    create_file, feature_review, generate_test_instructions, implement, plan_order, review,
};
use crate::context::{BudgetExhausted, Context};
use crate::traits::IssueType;

pub trait UserInteractor {
    fn prompt_text(&self, question: &str, default: Option<&str>) -> Result<String>;
    fn prompt_choice(&self, question: &str, choices: &[&str]) -> Result<usize>;
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

    fn prompt_choice(&self, question: &str, choices: &[&str]) -> Result<usize> {
        let answer = inquire::Select::new(question, choices.to_vec()).prompt()?;
        Ok(choices.iter().position(|&c| c == answer).unwrap())
    }

    fn prompt_confirm(&self, question: &str, default: bool) -> Result<bool> {
        Ok(inquire::Confirm::new(question)
            .with_default(default)
            .prompt()?)
    }
}

pub struct WizardOutput {
    pub issue_tracker_kind: String,
    pub repo: String,
    pub agent_kind: String,
    pub settings_file: Option<String>,
    pub context_file: Option<String>,
    pub commit_strategy: String,
}

impl WizardOutput {
    pub fn defaults() -> Self {
        Self {
            issue_tracker_kind: "github".to_string(),
            repo: "owner/repo".to_string(),
            agent_kind: "local".to_string(),
            settings_file: None,
            context_file: None,
            commit_strategy: "feature-branch".to_string(),
        }
    }
}

const ISSUE_TRACKER_KINDS: &[&str] = &["github"];
const AGENT_KINDS: &[&str] = &["local"];
const COMMIT_STRATEGIES: &[&str] = &["direct", "per-ticket", "feature-branch"];

pub fn interactive_config_wizard(
    base_dir: &std::path::Path,
    interactor: &dyn UserInteractor,
) -> Result<WizardOutput> {
    let tracker_idx = interactor.prompt_choice("Issue tracker", ISSUE_TRACKER_KINDS)?;
    let issue_tracker_kind = ISSUE_TRACKER_KINDS[tracker_idx].to_string();

    let repo = interactor.prompt_text("Repository (owner/repo)", Some("owner/repo"))?;

    let agent_idx = interactor.prompt_choice("Agent", AGENT_KINDS)?;
    let agent_kind = AGENT_KINDS[agent_idx].to_string();

    let settings_file = if interactor.prompt_confirm("Specify an agent settings file?", false)? {
        Some(interactor.prompt_text("Settings file path", Some(".claude/settings.json"))?)
    } else {
        None
    };

    let context_file =
        if interactor.prompt_confirm("Specify a context file (e.g. CLAUDE.md)?", false)? {
            Some(interactor.prompt_text("Context file path", Some("CLAUDE.md"))?)
        } else {
            None
        };

    let strategy_idx = interactor.prompt_choice("Commit strategy", COMMIT_STRATEGIES)?;
    let commit_strategy = COMMIT_STRATEGIES[strategy_idx].to_string();

    let output = WizardOutput {
        issue_tracker_kind,
        repo,
        agent_kind,
        settings_file,
        context_file,
        commit_strategy,
    };

    interactor.prompt_confirm(
        &format!(
            "Settings look good?\n  issue_tracker: {} | repo: {} | agent: {} | commit_strategy: {}",
            output.issue_tracker_kind, output.repo, output.agent_kind, output.commit_strategy
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
    let intern_dir = base_dir.join(".intern");
    let prompts_dir = intern_dir.join("prompts");
    create_file(
        &intern_dir.join("config.toml"),
        &generate_config_toml(wizard),
    )?;
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
    Ok(())
}

fn generate_config_toml(wizard: &WizardOutput) -> String {
    let mut lines = vec![];

    if let Some(ref ctx) = wizard.context_file {
        lines.push(format!("context_file = {:?}", ctx));
    }

    lines.push(String::new());
    lines.push("[issue_tracker]".to_string());
    lines.push(format!("kind = {:?}", wizard.issue_tracker_kind));
    lines.push(format!("repo = {:?}", wizard.repo));

    lines.push(String::new());
    lines.push("[agent]".to_string());
    lines.push(format!("kind = {:?}", wizard.agent_kind));
    if let Some(ref sf) = wizard.settings_file {
        lines.push(format!("settings_file = {:?}", sf));
    }

    lines.push(String::new());
    lines.push("[run]".to_string());
    lines.push("max_iterations = 100".to_string());
    lines.push(format!("commit_strategy = {:?}", wizard.commit_strategy));

    lines.push(String::new());
    lines.join("\n")
}

pub fn complete_ticket(issue_id: u64, ctx: &Context) -> Result<()> {
    loop {
        let result = (|| -> Result<bool> {
            implement(issue_id, ctx)?;
            review(issue_id, ctx)
        })();

        match result {
            Ok(false) => break,
            Ok(true) => continue,
            Err(e) if e.downcast_ref::<BudgetExhausted>().is_some() => {
                ctx.issues.skip_issue(issue_id)?;
                return Ok(());
            }
            Err(e) => return Err(e),
        }
    }
    generate_test_instructions(issue_id, ctx)?;
    Ok(())
}

pub fn complete_feature(issue_id: u64, ctx: &Context) -> Result<()> {
    let initial_children = ctx.issues.get_children(issue_id)?;
    let initial_ids: std::collections::HashSet<u64> =
        initial_children.iter().map(|i| i.id).collect();
    execute_ordered(&initial_children, ctx)?;

    let has_findings = feature_review(issue_id, ctx)?;
    if has_findings {
        let all_children = ctx.issues.get_children(issue_id)?;
        let new_children: Vec<_> = all_children
            .into_iter()
            .filter(|i| !initial_ids.contains(&i.id))
            .collect();
        execute_ordered(&new_children, ctx)?;

        if feature_review(issue_id, ctx)? {
            ctx.issues.skip_issue(issue_id)?;
            return Ok(());
        }
    }

    generate_test_instructions(issue_id, ctx)?;
    Ok(())
}

pub fn execute_ordered(issues: &[crate::traits::Issue], ctx: &Context) -> Result<()> {
    let ordered_ids = plan_order(issues, ctx)?;
    for id in ordered_ids {
        match ctx.issues.issue_type(id)? {
            IssueType::Ticket => complete_ticket(id, ctx)?,
            IssueType::Feature => complete_feature(id, ctx)?,
        }
    }
    Ok(())
}
