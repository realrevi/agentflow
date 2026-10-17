//! What the frontend's turn Channel (`turns_subscribe`) receives.

use serde::Serialize;

use super::signal::Action;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ActivityKind {
    Think,
    Tool,
    Wait,
    Idle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TurnEvent {
    /// A log file was matched to the panel. `exact`: the log marks turn ends (Claude Code,
    /// Codex); else the end comes from the terminal idle heuristic.
    Attached { panel: String, exact: bool },
    Activity {
        panel: String,
        kind: ActivityKind,
        action: Option<Action>,
        target: Option<String>,
    },
    /// The assistant text of the turn so far (only for logs without exact turn ends: it is
    /// the report when the terminal goes idle).
    Draft { panel: String, text: String },
    /// The turn ended; `report` is the final assistant message (empty if interrupted).
    TurnEnded {
        panel: String,
        report: String,
        interrupted: bool,
    },
}

/// Receives turn events (the Channel of the last `turns_subscribe`).
pub type TurnSink = Box<dyn Fn(TurnEvent) + Send + Sync>;
