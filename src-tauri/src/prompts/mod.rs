//! Prompt history: what was sent from each agent panel's prompt bar, kept per workspace in
//! its own JSON file in the app data dir (not in settings.json).

mod model;
mod store;

pub use model::PromptEntry;
pub use store::PromptHistory;
