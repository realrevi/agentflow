//! What the chat panel receives on its Channel while a reply streams.

use serde::Serialize;

use crate::conversations::ChatMessage;
use crate::providers::{EventSink, ProviderError, StreamEvent, TokenUsage};

/// Deltas and usage first, then exactly one `Stop` or `Error`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ChatEvent {
    Delta {
        text: String,
    },
    Usage {
        usage: TokenUsage,
    },
    /// The stored reply (`None` when nothing arrived before Stop was pressed).
    Stop {
        reason: String,
        message: Option<ChatMessage>,
    },
    /// The stored partial reply, if any text had arrived.
    Error {
        error: ProviderError,
        message: Option<ChatMessage>,
    },
}

/// Collects the reply and forwards it: the deltas of one network chunk go out as one
/// message, usage only when it changed.
pub struct ForwardingSink<F: FnMut(ChatEvent) + Send> {
    send: F,
    pub text: String,
    pending: String,
    pub usage: TokenUsage,
    usage_sent: TokenUsage,
}

impl<F: FnMut(ChatEvent) + Send> ForwardingSink<F> {
    pub fn new(send: F) -> Self {
        Self {
            send,
            text: String::new(),
            pending: String::new(),
            usage: TokenUsage::default(),
            usage_sent: TokenUsage::default(),
        }
    }

    pub fn send(&mut self, event: ChatEvent) {
        self.flush();
        (self.send)(event);
    }
}

impl<F: FnMut(ChatEvent) + Send> EventSink for ForwardingSink<F> {
    fn event(&mut self, event: StreamEvent) {
        match event {
            StreamEvent::Delta(text) => {
                self.text.push_str(&text);
                self.pending.push_str(&text);
            }
            StreamEvent::Usage(usage) => self.usage = usage,
        }
    }

    fn flush(&mut self) {
        if !self.pending.is_empty() {
            let text = std::mem::take(&mut self.pending);
            (self.send)(ChatEvent::Delta { text });
        }
        if self.usage != self.usage_sent {
            self.usage_sent = self.usage;
            (self.send)(ChatEvent::Usage { usage: self.usage });
        }
    }
}
