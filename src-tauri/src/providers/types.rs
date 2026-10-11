//! Data shared by the provider adapters, the chat session and stored conversations.

use serde::{Deserialize, Serialize};

/// Token counts of one reply. Cache fields stay 0 when the provider does not report them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Input tokens read from the provider's prompt cache.
    pub cache_read_tokens: u64,
    /// Input tokens written to the prompt cache (Anthropic reports these).
    pub cache_write_tokens: u64,
}
