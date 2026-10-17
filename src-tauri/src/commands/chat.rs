use tauri::ipc::Channel;
use tauri::State;

use crate::chat::{
    compact, start_chat, ChatDeps, ChatEvent, ChatManager, ChatSendRequest, ChatStarted,
    CompactRequest, CompactResult,
};
use crate::conversations::ConversationStore;
use crate::error::AppResult;
use crate::providers::KeyVault;
use crate::settings::SettingsStore;
use crate::tokens::UsageTracker;

/// Stores the user message (or prepares a retry) and starts streaming the reply to
/// `on_event`. Returns at once; the reply arrives on the channel. A hard budget stop fails
/// with kind `budget` unless `request.allowOnce` carries a token from `usage_allow_once`.
#[tauri::command]
pub async fn chat_send(
    settings: State<'_, SettingsStore>,
    store: State<'_, ConversationStore>,
    vault: State<'_, KeyVault>,
    manager: State<'_, ChatManager>,
    tracker: State<'_, UsageTracker>,
    request: ChatSendRequest,
    on_event: Channel<ChatEvent>,
) -> AppResult<ChatStarted> {
    let deps = deps(&settings, &store, &vault, &manager, &tracker).await;
    start_chat(deps, request, move |event| {
        // The panel may be gone (closed or reloaded); the reply is stored anyway.
        let _ = on_event.send(event);
    })
    .await
}

/// Stops a streaming reply. `false` if it had already ended.
#[tauri::command]
pub async fn chat_cancel(manager: State<'_, ChatManager>, request_id: u64) -> AppResult<bool> {
    Ok(manager.cancel(request_id))
}

/// Summarizes the older turns of a chat (see `chat/compact.rs`).
#[tauri::command]
pub async fn chat_compact(
    settings: State<'_, SettingsStore>,
    store: State<'_, ConversationStore>,
    vault: State<'_, KeyVault>,
    manager: State<'_, ChatManager>,
    tracker: State<'_, UsageTracker>,
    request: CompactRequest,
) -> AppResult<CompactResult> {
    let deps = deps(&settings, &store, &vault, &manager, &tracker).await;
    compact(deps, request).await
}

async fn deps(
    settings: &SettingsStore,
    store: &ConversationStore,
    vault: &KeyVault,
    manager: &ChatManager,
    tracker: &UsageTracker,
) -> ChatDeps {
    ChatDeps {
        settings: settings.get().await,
        store: store.clone(),
        vault: vault.clone(),
        manager: manager.clone(),
        tracker: tracker.clone(),
    }
}
