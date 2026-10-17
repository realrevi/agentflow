//! API chat conversations (phase 5): one JSON file per conversation, per workspace, in the
//! app data dir. Every assistant message carries a usage record for phase 6; agent messages
//! (phase 7) also carry their parts (reasoning, tool calls, tool results).

mod agent_state;
mod checks;
mod model;
mod store;
#[cfg(test)]
mod tests;

pub use checks::MAX_ATTACHMENT_BYTES;
pub use model::{
    AgentState, Attachment, ChatMessage, Compaction, Conversation, ConversationPatch,
    ConversationSummary, MessageStatus, PermissionMode, Role, UsageRecord, FORMAT_VERSION,
};
pub use store::ConversationStore;
