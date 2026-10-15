use tauri::ipc::Channel;
use tauri::State;

use crate::chat::{start_chat, ChatDeps, ChatEvent, ChatManager, ChatSendRequest, ChatStarted};
use crate::conversations::ConversationStore;
use crate::error::AppResult;
use crate::providers::KeyVault;
use crate::settings::SettingsStore;

/// Stores the user message (or prepares a retry) and starts streaming the reply to
/// `on_event`. Returns at once; the reply arrives on the channel.
#[tauri::command]
pub async fn chat_send(
    settings: State<'_, SettingsStore>,
    store: State<'_, ConversationStore>,
    vault: State<'_, KeyVault>,
    manager: State<'_, ChatManager>,
    request: ChatSendRequest,
    on_event: Channel<ChatEvent>,
) -> AppResult<ChatStarted> {
    let deps = ChatDeps {
        settings: settings.get().await,
        store: store.inner().clone(),
        vault: vault.inner().clone(),
        manager: manager.inner().clone(),
    };
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
