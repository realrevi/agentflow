//! API agents (phase 7): answering permission requests and per-turn checkpoints (list,
//! restore). The permission mode is part of the conversation (`conversation_update` with
//! `permissionMode`; `conversation_get` returns it under `agent`).

use std::path::Path;

use tauri::State;

use crate::agent::{AgentServices, CheckpointSummary, PermissionAnswer, RestoreReport};
use crate::error::AppResult;
use crate::fs::move_to_trash;
use crate::util::run_blocking;

/// The user's answer to a permission request. `false` if the request is gone (the turn
/// ended or was cancelled).
#[tauri::command]
pub async fn agent_permission_answer(
    agent: State<'_, AgentServices>,
    request_id: u64,
    answer: PermissionAnswer,
) -> AppResult<bool> {
    Ok(agent.gate.answer(request_id, answer))
}

/// Turns of a conversation that changed files, newest first.
#[tauri::command]
pub async fn agent_checkpoints(
    agent: State<'_, AgentServices>,
    workspace: Option<String>,
    conversation_id: String,
) -> AppResult<Vec<CheckpointSummary>> {
    let checkpoints = agent.checkpoints.clone();
    run_blocking(move || Ok(checkpoints.list(workspace.as_deref(), &conversation_id))).await
}

/// Undoes a turn's file changes: originals are written back, files it created go to the
/// OS trash.
#[tauri::command]
pub async fn agent_checkpoint_restore(
    agent: State<'_, AgentServices>,
    workspace: Option<String>,
    conversation_id: String,
    turn: String,
) -> AppResult<RestoreReport> {
    let checkpoints = agent.checkpoints.clone();
    run_blocking(move || {
        checkpoints.restore(
            workspace.as_deref(),
            &conversation_id,
            &turn,
            |path: &Path| move_to_trash(path),
        )
    })
    .await
}
