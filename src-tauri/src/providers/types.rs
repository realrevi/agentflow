//! Data shared by the provider adapters, the chat session and stored conversations.

use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// A tool the model may call: name, description and a JSON Schema for its input.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub schema: Value,
}

/// A tool call the model made. `input` is the parsed JSON object (an object with the raw
/// text under `"_raw"` when the model sent invalid JSON).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// The result of one tool call. `call_id`, `content` and `is_error` go to the model; the
/// rest is for the chat UI only (adapters never send it).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    pub call_id: String,
    pub content: String,
    #[serde(default)]
    pub is_error: bool,
    #[serde(default, skip_serializing_if = "ToolOutcome::is_ran")]
    pub outcome: ToolOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "ToolMeta::is_empty")]
    pub meta: ToolMeta,
}

/// Whether the call ran at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolOutcome {
    #[default]
    Ran,
    /// The user said no.
    Denied,
    /// Never ran: the turn was stopped, or the tool is not available in this mode.
    Skipped,
}

impl ToolOutcome {
    pub fn is_ran(&self) -> bool {
        *self == Self::Ran
    }
}

/// Numbers the chat builds its one-line summaries from (localized there).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolMeta {
    /// Entries (list_dir), files (glob), matches (search), lines (write_file),
    /// replacements (edit_file).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    /// Files with matches (search).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<u64>,
    /// The line range shown and the file's line count (read_file).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<[u64; 3]>,
    /// write_file: the file did not exist before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i64>,
}

impl ToolMeta {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    pub fn count(count: usize) -> Self {
        Self {
            count: Some(count as u64),
            ..Self::default()
        }
    }
}

/// Reasoning the model produced, kept to send back unchanged where the provider needs it:
/// Anthropic `thinking` (text + `signature`) or `redacted_thinking` (`redacted`), and
/// OpenAI-compatible `reasoning_content` / OpenRouter `reasoning_details` (`details`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Reasoning {
    pub text: String,
    pub signature: Option<String>,
    pub redacted: Option<String>,
    pub details: Option<Value>,
    /// How long the model thought (chat UI only; adapters never send it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// One piece of a turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Part {
    Text { text: String },
    Reasoning(Reasoning),
    ToolCall(ToolCall),
    ToolResult(ToolResult),
}

/// A piece of a turn. `cache` marks an Anthropic prompt-cache breakpoint at its end
/// (adapters without explicit caching ignore it).
#[derive(Debug, Clone, PartialEq)]
pub struct TurnBlock {
    pub part: Part,
    pub cache: bool,
}

impl TurnBlock {
    pub fn text(text: impl Into<String>) -> Self {
        Self::part(Part::Text { text: text.into() })
    }

    pub fn part(part: Part) -> Self {
        Self { part, cache: false }
    }

    /// The text of a text block ("" for other parts).
    pub fn text_str(&self) -> &str {
        match &self.part {
            Part::Text { text } => text,
            _ => "",
        }
    }

    pub fn text_mut(&mut self) -> Option<&mut String> {
        match &mut self.part {
            Part::Text { text } => Some(text),
            _ => None,
        }
    }

    /// Whether a cache breakpoint may sit at the end of this block (thinking blocks and
    /// empty text cannot carry one).
    pub fn cacheable(&self) -> bool {
        match &self.part {
            Part::Text { text } => !text.is_empty(),
            Part::ToolCall(_) | Part::ToolResult(_) => true,
            Part::Reasoning(_) => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatTurn {
    pub role: ChatRole,
    /// In order; text separators are part of the text.
    pub blocks: Vec<TurnBlock>,
}

impl ChatTurn {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            blocks: vec![TurnBlock::text(text)],
        }
    }

    /// The whole text of the turn (text blocks only).
    pub fn content(&self) -> String {
        self.blocks.iter().map(TurnBlock::text_str).collect()
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
    /// Tools offered to the model (empty = a plain chat).
    pub tools: Vec<ToolSpec>,
}

/// What an adapter reports while a reply streams.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    Delta(String),
    /// Reasoning text as it streams (shown, not stored: see `Reasoning`).
    ReasoningDelta(String),
    /// A finished reasoning block, to store and send back.
    Reasoning(Reasoning),
    /// A complete tool call (its streamed input fully assembled).
    ToolCall(ToolCall),
    /// Totals so far (later reports replace earlier ones).
    Usage(TokenUsage),
}

/// How a stream ended without an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEnd {
    /// The provider finished; its stop reason (e.g. `end_turn`, `stop`, `tool_use`).
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
    pub pricing: Option<Value>,
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

/// Parses a tool call's accumulated input JSON. Invalid or non-object JSON is kept as
/// `{"_raw": text}` so the tool can report it back to the model instead of failing.
pub fn parse_tool_input(text: &str) -> Value {
    if text.trim().is_empty() {
        return Value::Object(serde_json::Map::new());
    }
    match serde_json::from_str::<Value>(text) {
        Ok(value @ Value::Object(_)) => value,
        _ => serde_json::json!({ "_raw": text }),
    }
}
