use anyhow::Result;

use crate::actions::{create_file, feature_review, generate_test_instructions, implement, plan_order, review};
use crate::context::{BudgetExhausted, Context};
use crate::traits::IssueType;

const DEFAULT_CONFIG: &str = include_str!("../scaffold/config.toml");
const PROMPT_IMPLEMENT: &str = include_str!("../scaffold/prompts/implement.md");
const PROMPT_REVIEW: &str = include_str!("../scaffold/prompts/review.md");
const PROMPT_FEATURE_REVIEW: &str = include_str!("../scaffold/prompts/feature_review.md");
const PROMPT_PLAN_ORDER: &str = include_str!("../scaffold/prompts/plan_order.md");
const PROMPT_TEST_INSTRUCTIONS: &str = include_str!("../scaffold/prompts/test_instructions.md");

pub fn scaffold_intern_directory(base_dir: &std::path::Path) -> Result<()> {
    let intern_dir = base_dir.join(".intern");
    let prompts_dir = intern_dir.join("prompts");
    create_file(&intern_dir.join("config.toml"), DEFAULT_CONFIG)?;
    create_file(&prompts_dir.join("implement.md"), PROMPT_IMPLEMENT)?;
    create_file(&prompts_dir.join("review.md"), PROMPT_REVIEW)?;
    create_file(&prompts_dir.join("feature_review.md"), PROMPT_FEATURE_REVIEW)?;
    create_file(&prompts_dir.join("plan_order.md"), PROMPT_PLAN_ORDER)?;
    create_file(&prompts_dir.join("test_instructions.md"), PROMPT_TEST_INSTRUCTIONS)?;
    Ok(())
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
    let initial_ids: std::collections::HashSet<u64> = initial_children.iter().map(|i| i.id).collect();
    execute_ordered(&initial_children, ctx)?;

    let has_findings = feature_review(issue_id, ctx)?;
    if has_findings {
        let all_children = ctx.issues.get_children(issue_id)?;
        let new_children: Vec<_> = all_children.into_iter().filter(|i| !initial_ids.contains(&i.id)).collect();
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
