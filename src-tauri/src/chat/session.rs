//! Starts a turn: checks the chat's provider, model, key and budget, stores the user message,
//! then runs the agent loop (`agent::run_turn`) on a background task that forwards events
//! and stores every step (with its usage record) however the turn ends.

use serde::{Deserialize, Serialize};

use super::events::ChatEvent;
use super::manager::ChatManager;
use crate::agent::{run_turn, CheckpointStore, ModelCapabilities, PermissionGate, Turn};
use crate::conversations::{Attachment, ChatMessage, ConversationStore, Role};
use crate::error::{AppError, AppResult};
use crate::providers::{build_provider, ChatRequest, KeyVault, ProviderError};
use crate::settings::Settings;
use crate::tokens::UsageTracker;
use crate::util::run_blocking;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSendRequest {
    pub workspace: Option<String>,
    pub conversation_id: String,
    /// `None` = retry the last user message.
    pub text: Option<String>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    /// The chat panel, for per-panel usage.
    #[serde(default)]
    pub panel: Option<String>,
    /// A token from `usage_allow_once`: lets this send through a hard budget stop.
    #[serde(default)]
    pub allow_once: Option<String>,
    /// Appended to the system prompt for this turn (a workstream manager's instructions);
    /// never stored with the conversation.
    #[serde(default)]
    pub extra_system: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatStarted {
    pub request_id: u64,
    pub user_message: Option<ChatMessage>,
    /// The conversation's title (the first prompt names a new chat).
    pub title: String,
}

/// Longer extra system text is ignored (the manager instructions are ~2 KB).
const MAX_EXTRA_SYSTEM: usize = 32 * 1024;

/// Everything a turn needs from the app's managed state.
pub struct ChatDeps {
    pub settings: Settings,
    pub store: ConversationStore,
    pub vault: KeyVault,
    pub manager: ChatManager,
    pub tracker: UsageTracker,
    pub gate: PermissionGate,
    pub checkpoints: CheckpointStore,
    pub capabilities: ModelCapabilities,
}

pub async fn start_chat<F>(
    deps: ChatDeps,
    request: ChatSendRequest,
    send: F,
) -> AppResult<ChatStarted>
where
    F: FnMut(ChatEvent) + Send + 'static,
{
    let ChatDeps {
        settings,
        store,
        vault,
        manager,
        tracker,
        gate,
        checkpoints,
        capabilities,
    } = deps;
    let workspace = request.workspace.clone();
    let id = request.conversation_id.clone();
    let conversation = {
        let (store, workspace, id) = (store.clone(), workspace.clone(), id.clone());
        run_blocking(move || store.get(workspace.as_deref(), &id)).await?
    };
    let entry = settings
        .providers
        .iter()
        .find(|p| Some(&p.id) == conversation.provider_id.as_ref())
        .cloned()
        .ok_or_else(|| AppError::Provider("Choose a provider for this chat.".into()))?;
    if conversation.model.trim().is_empty() {
        return Err(AppError::Provider("Choose a model for this chat.".into()));
    }
    tracker.check_send(&entry.id, request.allow_once.as_deref())?;
    let key = {
        let (vault, provider_id) = (vault.clone(), entry.id.clone());
        run_blocking(move || vault.get(&provider_id)).await?
    };
    let secret = key.as_ref().map(|k| k.expose().to_owned());
    let provider = build_provider(&entry, key)?;
    let http = manager.http()?;

    let qualified = format!("{}|{id}", workspace.as_deref().unwrap_or(""));
    let (request_id, cancel) = manager.register(qualified)?;
    let panel = request.panel.clone();
    let extra_system = request
        .extra_system
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty() && text.len() <= MAX_EXTRA_SYSTEM)
        .map(str::to_owned);
    let stored = {
        let (store, workspace, id) = (store.clone(), workspace.clone(), id.clone());
        let ChatSendRequest {
            text, attachments, ..
        } = request;
        run_blocking(move || match text {
            Some(text) => store
                .append_user(workspace.as_deref(), &id, &text, attachments)
                .map(|(conversation, message)| (conversation, Some(message))),
            None => store
                .prepare_retry(workspace.as_deref(), &id)
                .map(|conversation| (conversation, None)),
        })
        .await
    };
    let (conversation, user_message) = match stored {
        Ok(stored) => stored,
        Err(err) => {
            manager.finish(request_id);
            return Err(err);
        }
    };
    let title = conversation.title.clone();
    // The turn is keyed by its user message (a retry reuses the last one).
    let turn_id = conversation
        .messages
        .iter()
        .rev()
        .find(|m| m.role == Role::User)
        .map(|m| m.id.clone())
        .unwrap_or_else(|| "turn".into());
    let turn = Turn {
        http,
        provider,
        entry,
        secret,
        workspace,
        conversation_id: id,
        panel,
        turn_id,
        extra_system,
        settings,
        store,
        tracker,
        gate,
        checkpoints,
        capabilities,
        cancel,
    };
    tauri::async_runtime::spawn(async move {
        run_turn(turn, send).await;
        manager.finish(request_id);
    });
    Ok(ChatStarted {
        request_id,
        user_message,
        title,
    })
}

/// For "Test connection": a one-line reply proves the key, URL and model work.
pub fn tiny_request(model: &str) -> ChatRequest {
    ChatRequest {
        model: model.to_owned(),
        system: String::new(),
        turns: vec![crate::providers::ChatTurn::user("Reply with OK.")],
        max_tokens: 16,
        temperature: None,
        caching: false,
        tools: Vec::new(),
    }
}

/// For errors raised before a request has a key at hand (config checks). Key-shaped text
/// is still masked, in case one ever ends up in such a message.
impl From<ProviderError> for AppError {
    fn from(err: ProviderError) -> Self {
        AppError::Provider(err.redacted(None).message)
    }
}
