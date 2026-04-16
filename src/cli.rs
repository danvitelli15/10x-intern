use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum MergeStrategyArg {
    Direct,
    FeatureBranch,
    PerTicket,
}

#[derive(Parser)]
#[command(name = "intern", about = "Autonomous ticket executor")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Implement a feature issue and its sub-issues
    Implement {
        /// GitHub issue ID
        issue_id: u64,
        /// Plan but don't execute agents
        #[arg(long)]
        dry_run: bool,
        /// Maximum number of agent invocations
        #[arg(long)]
        max_iterations: Option<u32>,
        /// How to handle branches and commits
        #[arg(long, value_enum)]
        merge_strategy: Option<MergeStrategyArg>,
    },
    /// Work through all issues matching a label
    Clear {
        /// GitHub label to query
        label: String,
        /// Plan but don't execute agents
        #[arg(long)]
        dry_run: bool,
        /// Maximum number of agent invocations
        #[arg(long)]
        max_iterations: Option<u32>,
        /// How to handle branches and commits
        #[arg(long, value_enum)]
        merge_strategy: Option<MergeStrategyArg>,
    },
    /// Run the review phase against a completed issue
    Review {
        /// GitHub issue ID
        issue_id: u64,
        /// Plan but don't execute agents
        #[arg(long)]
        dry_run: bool,
    },
    /// Scaffold .intern/config.toml and default prompt files
    Init {
        /// Use all defaults without prompting
        #[arg(long)]
        defaults: bool,
    },
}
