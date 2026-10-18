//! Turn detection and live activity for agent CLI panels (phase 9), read from the CLIs' own
//! session logs with the phase 6 readers (see docs/turn-signals.md): when a turn ends, its
//! final assistant message (the report) and the tool calls in between ("run npm test").
//! CLIs whose log has no turn end (Gemini) get drafts; the frontend's terminal idle
//! heuristic ends their turns, marked approximate. API chats report through their stream.

mod events;
mod fold;
mod parse;
mod signal;
mod watcher;

pub use events::TurnEvent;
pub use watcher::TurnWatcher;

#[cfg(test)]
mod tests;
