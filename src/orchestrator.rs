use anyhow::Result;

use crate::behaviors::TerminalInteractor;
use crate::cli::Command;
use crate::config::Config;
use crate::workflows::{build_context, clear_workflow, implement_workflow, init_workflow, init_workflow_with_defaults};

pub fn run(command: Command, config: Config) -> Result<()> {
    if let Command::Init { defaults } = command {
        let base_dir = config.resolve_work_directory();
        return if defaults {
            init_workflow_with_defaults(&base_dir)
        } else {
            init_workflow(&base_dir, &TerminalInteractor)
        };
    }
    let ctx = build_context(&command, &config)?;
    match command {
        Command::Implement { issue_id, .. } => implement_workflow(issue_id, &ctx),
        Command::Clear { label, .. } => clear_workflow(&label, &ctx),
        Command::Review { .. } => todo!("review"),
        Command::Init { .. } => unreachable!(),
    }
}
