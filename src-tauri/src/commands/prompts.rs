use tauri::State;

use crate::error::AppResult;
use crate::prompts::{PromptEntry, PromptHistory};
use crate::util::run_blocking;

/// The prompts sent from one panel in a workspace (`None` = no workspace), oldest first.
#[tauri::command]
pub async fn prompts_list(
    history: State<'_, PromptHistory>,
    workspace: Option<String>,
    panel: String,
) -> AppResult<Vec<PromptEntry>> {
    let history = history.inner().clone();
    run_blocking(move || history.list(workspace.as_deref(), &panel)).await
}

/// Records a sent prompt (capped at 200 per panel) and returns the stored entry.
#[tauri::command]
pub async fn prompts_add(
    history: State<'_, PromptHistory>,
    workspace: Option<String>,
    panel: String,
    text: String,
) -> AppResult<PromptEntry> {
    let history = history.inner().clone();
    run_blocking(move || history.add(workspace.as_deref(), &panel, &text)).await
}
