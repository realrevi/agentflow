//! Data shared by the provider adapters, the chat session and stored conversations.

use serde::{Deserialize, Serialize};

/// Token counts of one reply, normalized across providers. Cache fields stay 0 when the
/// provider does not report them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TokenUsage {
    /// Input tokens that were NOT read from the prompt cache (OpenAI-style `prompt_tokens`
    /// minus `cached_tokens`; Anthropic's `input_tokens`).
    pub input_tokens: u64,
    /// Output tokens, reasoning/thinking included.
    pub output_tokens: u64,
    /// Input tokens read from the provider's prompt cache.
    pub cache_read_tokens: u64,
    /// Input tokens written to the prompt cache, when reported.
    pub cache_write_tokens: u64,
}

impl TokenUsage {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
}

/// A piece of a turn's text. `cache` marks an Anthropic prompt-cache breakpoint at its end
/// (adapters without explicit caching ignore it and just concatenate the text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnBlock {
    pub text: String,
    pub cache: bool,
}

impl TurnBlock {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            cache: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatTurn {
    pub role: ChatRole,
    /// Concatenated in order; separators are part of the text.
    pub blocks: Vec<TurnBlock>,
}

impl ChatTurn {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            blocks: vec![TurnBlock::text(text)],
        }
    }

    /// The whole text of the turn.
    pub fn content(&self) -> String {
        self.blocks.iter().map(|b| b.text.as_str()).collect()
    }
}

/// One streaming chat call, already validated (turns alternate and start with the user).
#[derive(Debug, Clone, PartialEq)]
pub struct ChatRequest {
    pub model: String,
    /// Empty = none.
    pub system: String,
    pub turns: Vec<ChatTurn>,
    pub max_tokens: u32,
    /// `None` = the provider's default (newer models reject anything else).
    pub temperature: Option<f64>,
    /// Anthropic: a cache breakpoint on the system prompt.
    pub caching: bool,
}

/// What an adapter reports while a reply streams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    Delta(String),
    /// Totals so far (later reports replace earlier ones).
    Usage(TokenUsage),
}

/// How a stream ended without an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEnd {
    /// The provider finished; its stop reason (e.g. `end_turn`, `stop`, `max_tokens`).
    Stopped(String),
    /// The user pressed Stop.
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    /// A display name, when the provider has one.
    pub name: Option<String>,
    /// Max input tokens, when the models endpoint says (Anthropic `max_input_tokens`,
    /// OpenRouter `context_length`).
    pub context_window: Option<u64>,
    /// OpenRouter's raw `pricing` object (USD per token, as strings); not sent to the UI.
    #[serde(skip)]
    pub pricing: Option<serde_json::Value>,
}

impl ModelInfo {
    pub fn named(id: impl Into<String>, name: Option<String>) -> Self {
        Self {
            id: id.into(),
            name,
            context_window: None,
            pricing: None,
        }
    }
}

/// Receives stream events. `flush` is called after each network chunk, so a sink can
/// batch the deltas of one chunk into one message to the UI.
pub trait EventSink: Send {
    fn event(&mut self, event: StreamEvent);
    fn flush(&mut self) {}
}
