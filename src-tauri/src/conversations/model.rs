//! The on-disk shape of one API chat conversation (`<id>.json`).

use serde::{Deserialize, Serialize};

use crate::providers::TokenUsage;

pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// Tells a missing field (`None`) apart from an explicit `null` (`Some(None)`).
fn double_option<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}
