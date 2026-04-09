use crate::traits::{Event, EventSink};

/// Headless EventSink — emits events as structured log lines.
/// Use this for CI runs or single-ticket invocations.
pub struct LogReporter;

impl EventSink for LogReporter {
    fn emit(&self, event: Event) {
        log::info!("{:?}", event);
    }
}
