use tauri::State;

use crate::conversations::{
    Conversation, ConversationPatch, ConversationStore, ConversationSummary,
};
use crate::error::AppResult;
use crate::util::run_blocking;

/// The workspace's conversations, most recently updated first.
#[tauri::command]
pub async fn conversations_list(
    store: State<'_, ConversationStore>,
    workspace: Option<String>,
) -> AppResult<Vec<ConversationSummary>> {
    let store = store.inner().clone();
    run_blocking(move || store.list(workspace.as_deref())).await
}

#[tauri::command]
pub async fn conversation_get(
    store: State<'_, ConversationStore>,
    workspace: Option<String>,
    id: String,
) -> AppResult<Conversation> {
    let store = store.inner().clone();
    run_blocking(move || store.get(workspace.as_deref(), &id)).await
}

#[tauri::command]
pub async fn conversation_create(
    store: State<'_, ConversationStore>,
    workspace: Option<String>,
    provider_id: Option<String>,
    model: String,
    system_prompt: String,
) -> AppResult<Conversation> {
    let store = store.inner().clone();
    run_blocking(move || store.create(workspace.as_deref(), provider_id, &model, &system_prompt))
        .await
}

/// Renames it, or changes its provider, model or system prompt.
#[tauri::command]
pub async fn conversation_update(
    store: State<'_, ConversationStore>,
    workspace: Option<String>,
    id: String,
    patch: ConversationPatch,
) -> AppResult<ConversationSummary> {
    let store = store.inner().clone();
    run_blocking(move || store.update(workspace.as_deref(), &id, patch)).await
}

/// Moves it to the app's trash folder (kept for 30 days).
#[tauri::command]
pub async fn conversation_delete(
    store: State<'_, ConversationStore>,
    workspace: Option<String>,
    id: String,
) -> AppResult<()> {
    let store = store.inner().clone();
    run_blocking(move || store.delete(workspace.as_deref(), &id)).await
}
