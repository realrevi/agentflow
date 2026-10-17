//! The on-disk shape of one API chat conversation (`<id>.json`).

use serde::{Deserialize, Serialize};

use crate::providers::{Part, TokenUsage};

/// v2 (phase 7): messages may carry `parts` (reasoning, tool calls, tool results), a `tool`
/// role and the agent state. v1 files load unchanged (the new fields default).
pub const FORMAT_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    /// Tool results of an agent step (sent to the model as the user's turn).
    Tool,
}

/// What an agent may do without asking (per conversation).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Reads run, writes and commands ask.
    #[default]
    Ask,
    /// Reads and writes run, commands ask.
    AutoEdit,
    /// Only read tools are offered.
    ReadOnly,
}

/// The agent's state in one conversation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AgentState {
    /// `None` = the default from settings.
    pub permission_mode: Option<PermissionMode>,
    /// Tools the user allowed "for this conversation" (dangerous commands still ask).
    pub allowed_tools: Vec<String>,
    /// The provider rejected tools for this chat's model: it continues as a plain chat.
    pub tools_disabled: bool,
    /// The environment part of the default system prompt, built once so the prompt (and
    /// the cached prefix) stays the same for the whole conversation.
    pub context: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageStatus {
    Complete,
    /// Stopped by the user; `content` holds what had arrived.
    Cancelled,
    /// The request failed; `content` holds what had arrived, `error` says why.
    Error,
}

/// A workspace file sent with a user message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    /// Workspace-relative, with `/`.
    pub path: String,
    pub content: String,
}

/// Recorded with every assistant reply that reported usage (phase 6 aggregates these).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRecord {
    pub provider_id: String,
    pub preset: String,
    pub model: String,
    /// When the reply finished (ms since the Unix epoch).
    pub at: u64,
    #[serde(flatten)]
    pub tokens: TokenUsage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub role: Role,
    pub content: String,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    pub at: u64,
    pub status: MessageStatus,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub usage: Option<UsageRecord>,
    /// Agent messages: reasoning, text and tool calls (assistant) or tool results (tool).
    /// Empty for plain chat messages, whose text is `content`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<Part>,
    /// Assistant messages of an agent: a digest of the system prompt and tools they were
    /// produced under (Anthropic thinking is only replayed under the same prefix).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Conversation {
    pub version: u32,
    pub id: String,
    /// Empty until the first prompt names it (or the user renames it).
    pub title: String,
    /// The workspace folder (informational; the folder name is derived from it).
    pub workspace: Option<String>,
    pub provider_id: Option<String>,
    pub model: String,
    pub system_prompt: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub messages: Vec<ChatMessage>,
    /// Set by Compact: requests send `summary` instead of the messages up to `through`.
    pub compaction: Option<Compaction>,
    pub agent: AgentState,
}

impl Conversation {
    /// Brings an older file up to `FORMAT_VERSION` (v1 -> v2 only adds defaults).
    pub fn migrated(mut self) -> Self {
        if self.version < FORMAT_VERSION {
            self.version = FORMAT_VERSION;
        }
        self
    }
}

/// A summary of the older part of a conversation (phase 6). The messages stay in the file
/// (the panel still shows them); only requests replace them with the summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Compaction {
    pub summary: String,
    /// Id of the last message the summary covers.
    pub through: String,
    pub at: u64,
    /// Provider entry and model that wrote the summary.
    pub provider_id: String,
    pub model: String,
}

impl Default for Conversation {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            id: String::new(),
            title: String::new(),
            workspace: None,
            provider_id: None,
            model: String::new(),
            system_prompt: String::new(),
            created_at: 0,
            updated_at: 0,
            messages: Vec::new(),
            compaction: None,
            agent: AgentState::default(),
        }
    }
}

/// What the sidebar lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    pub id: String,
    pub title: String,
    pub provider_id: Option<String>,
    pub model: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub message_count: usize,
}

impl From<&Conversation> for ConversationSummary {
    fn from(conversation: &Conversation) -> Self {
        Self {
            id: conversation.id.clone(),
            title: conversation.title.clone(),
            provider_id: conversation.provider_id.clone(),
            model: conversation.model.clone(),
            created_at: conversation.created_at,
            updated_at: conversation.updated_at,
            message_count: conversation.messages.len(),
        }
    }
}

/// Header and sidebar edits. `provider_id: Some(None)` clears the provider.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ConversationPatch {
    pub title: Option<String>,
    #[serde(deserialize_with = "double_option")]
    pub provider_id: Option<Option<String>>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    /// The agent's permission mode for this chat.
    pub permission_mode: Option<PermissionMode>,
}

/// Tells a missing field (`None`) apart from an explicit `null` (`Some(None)`).
fn double_option<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}
