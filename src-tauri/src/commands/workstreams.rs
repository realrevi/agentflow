//! The workstream log (phase 9): handoffs appended as they happen, read by the Timeline.

use tauri::State;

use crate::error::AppResult;
use crate::util::run_blocking;
use crate::workstreams::{HandoffEntry, WorkstreamLog};

#[tauri::command]
pub async fn workstream_log_append(
    log: State<'_, WorkstreamLog>,
    workspace: Option<String>,
    entry: HandoffEntry,
) -> AppResult<()> {
    let log = log.inner().clone();
    run_blocking(move || log.append(workspace.as_deref(), entry)).await
}

/// One workstream's handoffs, oldest first.
#[tauri::command]
pub async fn workstream_log_read(
    log: State<'_, WorkstreamLog>,
    workspace: Option<String>,
    workstream: String,
) -> AppResult<Vec<HandoffEntry>> {
    let log = log.inner().clone();
    run_blocking(move || log.read(workspace.as_deref(), &workstream)).await
}
