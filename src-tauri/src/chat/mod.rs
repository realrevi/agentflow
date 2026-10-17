//! API chat replies (phase 5): build the request from a stored conversation, stream it
//! through a provider adapter to the panel's Channel, and store the reply with its usage.

mod cache;
mod events;
mod manager;
mod request;
mod session;
#[cfg(test)]
mod tests;

pub use events::ChatEvent;
pub use manager::ChatManager;
pub use session::{start_chat, tiny_request, ChatDeps, ChatSendRequest, ChatStarted};
