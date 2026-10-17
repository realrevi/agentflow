//! API chat conversations (phase 5): one JSON file per conversation, per workspace, in the
//! app data dir. Every assistant message carries a usage record for phase 6.

mod checks;
mod model;
mod store;
#[cfg(test)]
mod tests;

pub use checks::MAX_ATTACHMENT_BYTES;
pub use model::{
    Attachment, ChatMessage, Compaction, Conversation, ConversationPatch, ConversationSummary,
    MessageStatus, Role, UsageRecord,
};
pub use store::ConversationStore;
