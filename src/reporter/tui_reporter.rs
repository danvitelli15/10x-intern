use crate::traits::{Event, EventSink};

/// Interactive TUI EventSink — renders progress using ratatui.
/// Placeholder: implement when the TUI phase begins.
pub struct TuiReporter;

impl EventSink for TuiReporter {
    fn emit(&self, event: Event) {
        todo!("render event in ratatui TUI")
    }
}
