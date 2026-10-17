//! API chats (phase 5) and agent turns (phase 7): build the request from a stored
//! conversation, start the turn (the loop is `agent::run_turn`), stream its events to the
//! panel's Channel; Compact and the pre-send estimate.

mod attachments;
mod cache;
mod compact;
mod estimate;
mod events;
mod manager;
mod request;
mod session;
#[cfg(test)]
mod tests;

pub use compact::{compact, CompactRequest, CompactResult};
pub use estimate::{estimate, EstimateRequest};
pub use events::{ChatEvent, Phase, StepSink};
pub use manager::ChatManager;
pub use request::{build_with, RequestOptions};
pub use session::{start_chat, tiny_request, ChatDeps, ChatSendRequest, ChatStarted};
