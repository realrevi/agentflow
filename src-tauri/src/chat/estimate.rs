//! The pre-send estimate: input tokens of the request the prompt bar would send now (history,
//! attachments, the draft), its input cost and how full the context window gets.

use serde::Deserialize;

use super::request::build_request;
use crate::conversations::{Attachment, ChatMessage, Conversation, MessageStatus, Role};
use crate::error::{AppError, AppResult};
use crate::settings::Settings;
use crate::tokens::{is_exact, request_tokens, Estimate, UsageTracker};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateRequest {
    pub workspace: Option<String>,
    pub conversation_id: String,
    /// The draft prompt (empty = just the history).
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

pub fn estimate(
    settings: &Settings,
    tracker: &UsageTracker,
    conversation: &Conversation,
    text: String,
    attachments: Vec<Attachment>,
) -> AppResult<Estimate> {
    let entry = settings
        .providers
        .iter()
        .find(|p| Some(&p.id) == conversation.provider_id.as_ref())
        .ok_or_else(|| AppError::Provider("Choose a provider for this chat.".into()))?;
    let mut draft = conversation.clone();
    draft.messages.push(ChatMessage {
        id: "draft".into(),
        role: Role::User,
        content: text,
        attachments,
        at: 0,
        status: MessageStatus::Complete,
        stop_reason: None,
        error: None,
        usage: None,
        parts: Vec::new(),
        prefix: None,
    });
    let chat = build_request(&draft, entry, false)?;
    let tokens = request_tokens(&chat);
    let model = &conversation.model;
    let (context_window, window_source) = match tracker.context_window(&entry.id, model) {
        Some(window) => (window, "endpoint"),
        None => match settings.chat.window_for(model) {
            Some(window) => (u64::from(window), "setting"),
            None => (u64::from(settings.chat.default_context_window), "default"),
        },
    };
    let share = tokens as f64 / context_window.max(1) as f64;
    let threshold = settings.chat.compact_threshold;
    Ok(Estimate {
        tokens,
        approximate: !is_exact(&entry.preset, model),
        cost: tracker
            .price_for(&entry.id, model)
            .map(|price| tokens as f64 * price.input / 1_000_000.0),
        context_window,
        window_source,
        share,
        threshold,
        should_compact: settings.chat.auto_compact && share >= threshold,
    })
}
