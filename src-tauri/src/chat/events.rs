//! What the chat panel receives on its Channel while an agent turn runs, and the sink that
//! collects one model call (text, reasoning, tool calls) and forwards it.

use std::time::Instant;

use serde::Serialize;

use crate::agent::PermissionRequest;
use crate::conversations::ChatMessage;
use crate::providers::{
    EventSink, Part, ProviderError, StreamEvent, TokenUsage, ToolCall, ToolResult,
};

/// What the character does right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    /// A model call is running.
    Think,
    /// A tool runs.
    Work,
    /// Waiting for the user's approval.
    Wait,
}

/// Deltas, usage and step messages first, then exactly one `Stop` or `Error`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ChatEvent {
    Delta {
        text: String,
    },
    /// Reasoning text as it streams (shown collapsed).
    Reasoning {
        text: String,
    },
    Usage {
        usage: TokenUsage,
    },
    /// For the character: a model call, a tool run, waiting for the user's answer.
    Status {
        phase: Phase,
    },
    /// A call starts running (after its permission, if it needed one).
    ToolStart {
        call: ToolCall,
    },
    /// Every call ends with one (ran, denied or skipped), before its `tool` message.
    ToolEnd {
        result: ToolResult,
    },
    /// Answer with `agent_permission_answer`.
    Permission {
        request: PermissionRequest,
    },
    /// `toolsUnsupported` (the chat continues without tools), `maxIterations`.
    Notice {
        kind: String,
        message: String,
    },
    /// A stored step: the assistant's tool calls, or the tool results.
    Message {
        message: ChatMessage,
    },
    /// The final reply (`None` when nothing arrived before Stop was pressed, or the turn
    /// ended after a tool step).
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

/// Collects one model call and forwards it: the deltas of one network chunk go out as one
/// message, usage only when it changed.
pub struct StepSink<'a> {
    send: &'a mut (dyn FnMut(ChatEvent) + Send),
    pending: String,
    pending_reasoning: String,
    /// From the first reasoning delta to the first answer text or tool call.
    thinking: Option<(Instant, Option<Instant>)>,
    pub text: String,
    /// In arrival order; text deltas extend the last text part.
    pub parts: Vec<Part>,
    pub usage: TokenUsage,
    usage_sent: TokenUsage,
}

impl<'a> StepSink<'a> {
    pub fn new(send: &'a mut (dyn FnMut(ChatEvent) + Send)) -> Self {
        Self {
            send,
            pending: String::new(),
            pending_reasoning: String::new(),
            thinking: None,
            text: String::new(),
            parts: Vec::new(),
            usage: TokenUsage::default(),
            usage_sent: TokenUsage::default(),
        }
    }

    /// Nothing arrived (no text, reasoning or tool call).
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    pub fn tool_calls(&self) -> Vec<ToolCall> {
        self.parts
            .iter()
            .filter_map(|part| match part {
                Part::ToolCall(call) => Some(call.clone()),
                _ => None,
            })
            .collect()
    }

    /// Reasoning that carries nothing to replay or show is dropped.
    fn keep_reasoning(reasoning: &crate::providers::Reasoning) -> bool {
        !reasoning.text.is_empty()
            || reasoning.signature.is_some()
            || reasoning.redacted.is_some()
            || reasoning.details.is_some()
    }
}

impl EventSink for StepSink<'_> {
    fn event(&mut self, event: StreamEvent) {
        if let (Some((_, end @ None)), StreamEvent::Delta(_) | StreamEvent::ToolCall(_)) =
            (&mut self.thinking, &event)
        {
            *end = Some(Instant::now());
        }
        match event {
            StreamEvent::Delta(text) => {
                self.text.push_str(&text);
                self.pending.push_str(&text);
                match self.parts.last_mut() {
                    Some(Part::Text { text: last }) => last.push_str(&text),
                    _ => self.parts.push(Part::Text { text }),
                }
            }
            StreamEvent::ReasoningDelta(text) => {
                self.thinking.get_or_insert((Instant::now(), None));
                self.pending_reasoning.push_str(&text);
            }
            StreamEvent::Reasoning(mut reasoning) => {
                if let Some((start, end)) = self.thinking {
                    let took = end.unwrap_or_else(Instant::now).duration_since(start);
                    reasoning.duration_ms = u64::try_from(took.as_millis()).ok();
                }
                if Self::keep_reasoning(&reasoning) {
                    self.parts.push(Part::Reasoning(reasoning));
                }
            }
            StreamEvent::ToolCall(call) => self.parts.push(Part::ToolCall(call)),
            StreamEvent::Usage(usage) => self.usage = usage,
        }
    }

    fn flush(&mut self) {
        if !self.pending_reasoning.is_empty() {
            let text = std::mem::take(&mut self.pending_reasoning);
            (self.send)(ChatEvent::Reasoning { text });
        }
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
