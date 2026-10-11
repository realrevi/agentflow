//! Starts a reply: checks the chat's provider, model and key, stores the user message, then
//! streams on a background task that forwards events and stores the assistant message
//! (with its usage record) however the stream ends.

use serde::{Deserialize, Serialize};

use super::events::{ChatEvent, ForwardingSink};
use super::manager::ChatManager;
use super::request::build_request;
use crate::conversations::{
    Attachment, ChatMessage, ConversationStore, MessageStatus, Role, UsageRecord,
};
use crate::error::{AppError, AppResult};
use crate::providers::{
    build_provider, ChatRequest, Http, KeyVault, Provider, ProviderError, StreamEnd,
};
use crate::settings::{ProviderEntry, Settings};
use crate::util::{now_ms, run_blocking};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSendRequest {
    pub workspace: Option<String>,
    pub conversation_id: String,
    /// `None` = retry the last user message.
    pub text: Option<String>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatStarted {
    pub request_id: u64,
    pub user_message: Option<ChatMessage>,
    /// The conversation's title (the first prompt names a new chat).
    pub title: String,
}

/// Everything a reply needs from the app's managed state.
pub struct ChatDeps {
    pub settings: Settings,
    pub store: ConversationStore,
    pub vault: KeyVault,
    pub manager: ChatManager,
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
    let key = {
        let (vault, provider_id) = (vault.clone(), entry.id.clone());
        run_blocking(move || vault.get(&provider_id)).await?
    };
    let secret = key.as_ref().map(|k| k.expose().to_owned());
    let provider = build_provider(&entry, key)?;
    let http = manager.http()?;

    let qualified = format!("{}|{id}", workspace.as_deref().unwrap_or(""));
    let (request_id, cancel) = manager.register(qualified)?;
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
    let chat = match build_request(&conversation, &entry) {
        Ok(chat) => chat,
        Err(err) => {
            manager.finish(request_id);
            return Err(err);
        }
    };

    let job = Job {
        workspace,
        conversation_id: id,
        entry,
        secret,
        provider,
        chat,
    };
    tauri::async_runtime::spawn(async move {
        job.run(http, store, cancel, send).await;
        manager.finish(request_id);
    });
    Ok(ChatStarted {
        request_id,
        user_message,
        title,
    })
}

struct Job {
    workspace: Option<String>,
    conversation_id: String,
    entry: ProviderEntry,
    /// Only to redact it from error messages.
    secret: Option<String>,
    provider: Box<dyn Provider>,
    chat: ChatRequest,
}

impl Job {
    async fn run<F>(
        self,
        http: Http,
        store: ConversationStore,
        cancel: crate::providers::CancelToken,
        send: F,
    ) where
        F: FnMut(ChatEvent) + Send,
    {
        let mut sink = ForwardingSink::new(send);
        let outcome = self
            .provider
            .stream_chat(&http, &self.chat, &mut sink, &cancel)
            .await
            .map_err(|err| err.redacted(self.secret.as_deref()));
        let usage = (!sink.usage.is_empty()).then(|| UsageRecord {
            provider_id: self.entry.id.clone(),
            preset: self.entry.preset.clone(),
            model: self.chat.model.clone(),
            at: now_ms(),
            tokens: sink.usage,
        });
        let (status, stop_reason, error) = match &outcome {
            Ok(StreamEnd::Stopped(reason)) => (MessageStatus::Complete, Some(reason.clone()), None),
            Ok(StreamEnd::Cancelled) => (MessageStatus::Cancelled, Some("cancelled".into()), None),
            Err(err) => (MessageStatus::Error, None, Some(err.message.clone())),
        };
        let keep = status == MessageStatus::Complete || !sink.text.trim().is_empty();
        let message = keep.then(|| ChatMessage {
            id: format!("a{:x}", now_ms()),
            role: Role::Assistant,
            content: std::mem::take(&mut sink.text),
            attachments: Vec::new(),
            at: now_ms(),
            status,
            stop_reason: stop_reason.clone(),
            error,
            usage,
        });
        if let Some(message) = message.clone() {
            let (workspace, id) = (self.workspace.clone(), self.conversation_id.clone());
            let saved =
                run_blocking(move || store.append_assistant(workspace.as_deref(), &id, message))
                    .await;
            if let Err(err) = saved {
                tracing::warn!("could not store the reply: {err}");
            }
        }
        sink.send(match outcome {
            Ok(_) => ChatEvent::Stop {
                reason: stop_reason.unwrap_or_default(),
                message,
            },
            Err(error) => ChatEvent::Error { error, message },
        });
    }
}

/// For "Test connection": a one-line reply proves the key, URL and model work.
pub fn tiny_request(model: &str) -> ChatRequest {
    ChatRequest {
        model: model.to_owned(),
        system: String::new(),
        turns: vec![crate::providers::ChatTurn {
            role: crate::providers::ChatRole::User,
            content: "Reply with OK.".into(),
        }],
        max_tokens: 16,
        temperature: None,
    }
}

/// For errors raised before a request has a key at hand (config checks). Key-shaped text
/// is still masked, in case one ever ends up in such a message.
impl From<ProviderError> for AppError {
    fn from(err: ProviderError) -> Self {
        AppError::Provider(err.redacted(None).message)
    }
}
