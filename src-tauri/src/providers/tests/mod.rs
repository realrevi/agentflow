//! Provider tests: fixture streams through both decoders, and full requests against a
//! local mock server (never a real provider, never a real key).

mod decoders;
mod http;
pub(crate) mod mock;

use super::types::{EventSink, StreamEvent, TokenUsage};

/// Collects what an adapter reports.
#[derive(Default)]
pub struct Collect {
    pub text: String,
    pub usage: Vec<TokenUsage>,
    pub flushes: usize,
}

impl EventSink for Collect {
    fn event(&mut self, event: StreamEvent) {
        match event {
            StreamEvent::Delta(text) => self.text.push_str(&text),
            StreamEvent::Usage(usage) => self.usage.push(usage),
        }
    }

    fn flush(&mut self) {
        self.flushes += 1;
    }
}

impl Collect {
    pub fn last_usage(&self) -> TokenUsage {
        self.usage.last().copied().unwrap_or_default()
    }
}
