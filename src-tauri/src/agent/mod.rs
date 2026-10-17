//! API agents (phase 7): the agent loop (model call -> tool calls -> permission -> run ->
//! results -> next call, until the model stops), the tools, permissions, the dangerous
//! command list, the default system prompt and per-turn checkpoints for undo.

mod capabilities;
mod checkpoint;
mod dangerous;
mod execute;
mod permissions;
mod prompt;
mod run;
mod step;
pub mod tools;

pub use capabilities::ModelCapabilities;
pub use checkpoint::{CheckpointStore, CheckpointSummary, RestoreReport};
pub use permissions::{PermissionAnswer, PermissionGate, PermissionRequest};
pub use run::{run_turn, Turn};

/// Managed state of the agents: pending permission requests, checkpoints and the
/// remembered "supports tools" flags.
#[derive(Clone)]
pub struct AgentServices {
    pub gate: PermissionGate,
    pub checkpoints: CheckpointStore,
    pub capabilities: ModelCapabilities,
}
