use crate::traits::{Event, EventSink};

/// Headless EventSink — emits events as structured log lines.
/// Use this for CI runs or single-ticket invocations.
pub struct LogReporter;

impl EventSink for LogReporter {
    fn emit(&self, event: Event) {
        match event {
            Event::IssueClaimed(id) => log::info!("issue #{id} claimed"),
            Event::IssueComplete(id) => log::info!("issue #{id} complete"),
            Event::AgentStarted(id) => log::info!("agent started for issue #{id}"),
            Event::AgentFinished { issue_id, success } => {
                if success {
                    log::info!("agent finished for issue #{issue_id} — success");
                } else {
                    log::info!("agent finished for issue #{issue_id} — failed");
                }
            }
            Event::ReviewStarted => log::info!("review started"),
            Event::ReviewComplete { issues_created } => {
                if issues_created > 0 {
                    log::info!("review complete — {issues_created} issue(s) created");
                } else {
                    log::info!("review complete — no issues");
                }
            }
            Event::RunComplete => log::info!("run complete"),
        }
    }
}
