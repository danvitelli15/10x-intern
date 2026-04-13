use anyhow::Result;

use crate::cli::Command;
use crate::config::Config;
use crate::workflows::{build_context, clear_workflow, implement_workflow, init_workflow};

pub fn run(command: Command, config: Config) -> Result<()> {
    if let Command::Init = command {
        return init_workflow(&config.resolve_work_directory());
    }
    let ctx = build_context(&command, &config)?;
    match command {
        Command::Implement { issue_id, .. } => implement_workflow(issue_id, &ctx),
        Command::Clear { label, .. } => clear_workflow(&label, &ctx),
        Command::Review { .. } => todo!("review"),
        Command::Init => unreachable!(),
    }
}
