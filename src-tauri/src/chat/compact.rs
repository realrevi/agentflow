//! Compact: the older part of a chat becomes a summary written by the model the user chose
//! (`chat.compactProviderId` / `compactModel`, else the chat's own). The last
//! `chat.compactKeepTurns` turns (a prompt and its replies) stay verbatim. The summary call's
//! usage goes to the ledger as `kind: "compact"`.

use serde::{Deserialize, Serialize};

use super::request::{requested_messages, user_content};
use super::session::ChatDeps;
use crate::conversations::{ChatMessage, Compaction, Conversation, Role};
use crate::error::{AppError, AppResult};
use crate::providers::{
    build_provider, ChatRequest, ChatTurn, EventSink, Part, StreamEvent, TokenUsage,
};
use crate::settings::{ProviderEntry, Settings};
use crate::tokens::{Source, UsageRecord};
use crate::util::{now_ms, run_blocking};

const SYSTEM: &str = "You condense a chat between a user and an AI assistant so it can \
continue without the full history. Write a compact summary that keeps every fact, decision, \
requirement, file name, code identifier and open question the rest of the chat may need. \
Leave out greetings and repetition. Reply with the summary only.";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactRequest {
    pub workspace: Option<String>,
    pub conversation_id: String,
    #[serde(default)]
    pub panel: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactResult {
    pub compaction: Compaction,
    /// How many messages the summary replaces (in requests; the chat still shows them).
    pub summarized: usize,
    pub usage: TokenUsage,
}

pub async fn compact(deps: ChatDeps, request: CompactRequest) -> AppResult<CompactResult> {
    let ChatDeps {
        settings,
        store,
        vault,
        manager,
        tracker,
        ..
    } = deps;
    let workspace = request.workspace.clone();
    let id = request.conversation_id.clone();
    let conversation = {
        let (store, workspace, id) = (store.clone(), workspace.clone(), id.clone());
        run_blocking(move || store.get(workspace.as_deref(), &id)).await?
    };
    let older = older_messages(&conversation, settings.chat.compact_keep_turns as usize)?;
    let through = older.last().map(|m| m.id.clone()).unwrap_or_default();
    let summarized = older.len();
    let (entry, model) = summarizer(&settings, &conversation)?;
    tracker.check_send(&entry.id, None)?;
    let prompt = transcript(conversation.compaction.as_ref(), &older);

    let key = {
        let (vault, provider_id) = (vault.clone(), entry.id.clone());
        run_blocking(move || vault.get(&provider_id)).await?
    };
    let secret = key.as_ref().map(|k| k.expose().to_owned());
    let provider = build_provider(&entry, key)?;
    let http = manager.http()?;
    // Reserves the chat: no reply streams while its history is being summarized.
    let qualified = format!("{}|{id}", workspace.as_deref().unwrap_or(""));
    let (request_id, cancel) = manager.register(qualified)?;
    let chat = ChatRequest {
        model: model.clone(),
        system: SYSTEM.into(),
        turns: vec![ChatTurn::user(prompt)],
        max_tokens: entry.max_tokens,
        temperature: None,
        caching: false,
        tools: Vec::new(),
    };
    let mut sink = Collect::default();
    let outcome = provider.stream_chat(&http, &chat, &mut sink, &cancel).await;
    manager.finish(request_id);
    let at = now_ms();
    if !sink.usage.is_empty() {
        let record = UsageRecord {
            at,
            source: Source::Api,
            provider: entry.id.clone(),
            preset: entry.preset.clone(),
            model: model.clone(),
            workspace: workspace.clone(),
            panel: request.panel.clone(),
            id: format!("compact:{id}:{at}"),
            kind: "compact".into(),
            tokens: sink.usage,
        };
        let tracker = tracker.clone();
        if let Err(err) = run_blocking(move || tracker.record(vec![record])).await {
            tracing::warn!("could not record usage: {err}");
        }
    }
    outcome.map_err(|err| AppError::Provider(err.redacted(secret.as_deref()).message))?;
    let summary = sink.text.trim().to_owned();
    if summary.is_empty() {
        return Err(AppError::Provider(
            "The model returned an empty summary.".into(),
        ));
    }
    let compaction = Compaction {
        summary,
        through,
        at,
        provider_id: entry.id,
        model,
    };
    let stored = compaction.clone();
    run_blocking(move || store.set_compaction(workspace.as_deref(), &id, stored)).await?;
    Ok(CompactResult {
        compaction,
        summarized,
        usage: sink.usage,
    })
}

/// The messages before the last `keep` turns (since the previous compaction).
pub fn older_messages(conversation: &Conversation, keep: usize) -> AppResult<Vec<ChatMessage>> {
    let messages = requested_messages(conversation);
    let starts: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role == Role::User)
        .map(|(i, _)| i)
        .collect();
    let cut = starts
        .len()
        .checked_sub(keep.max(1))
        .and_then(|first_kept| starts.get(first_kept))
        .copied()
        .filter(|cut| *cut > 0)
        .ok_or_else(|| AppError::InvalidInput("nothing to compact yet".into()))?;
    Ok(messages[..cut].iter().map(|m| (*m).clone()).collect())
}

/// The provider entry and model that write the summary.
fn summarizer(
    settings: &Settings,
    conversation: &Conversation,
) -> AppResult<(ProviderEntry, String)> {
    let chosen = settings.chat.compact_provider_id.as_ref();
    let provider_id = chosen.or(conversation.provider_id.as_ref());
    let entry = settings
        .providers
        .iter()
        .find(|p| Some(&p.id) == provider_id)
        .cloned()
        .ok_or_else(|| AppError::Provider("Choose a provider for this chat.".into()))?;
    let own_provider = conversation.provider_id.as_ref() == Some(&entry.id);
    let model = if !settings.chat.compact_model.is_empty() {
        settings.chat.compact_model.clone()
    } else if own_provider {
        conversation.model.clone()
    } else {
        entry.default_model.clone()
    };
    if model.trim().is_empty() {
        return Err(AppError::Provider("Choose a model for Compact.".into()));
    }
    Ok((entry, model))
}

/// Long tool outputs are clipped in the transcript; the summary needs their gist only.
const TRANSCRIPT_RESULT_CHARS: usize = 2_000;

fn clip_chars(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_owned(),
    }
}

fn transcript(previous: Option<&Compaction>, older: &[ChatMessage]) -> String {
    let mut out = String::new();
    if let Some(previous) = previous {
        out.push_str("Summary of the chat before this part:\n\n");
        out.push_str(previous.summary.trim());
        out.push_str("\n\n");
    }
    out.push_str("Chat to summarize:\n\n");
    for message in older {
        match message.role {
            Role::User => {
                out.push_str("User:\n");
                out.push_str(&user_content(&message.content, &message.attachments));
            }
            Role::Assistant => {
                out.push_str("Assistant:\n");
                out.push_str(&message.content);
                for part in &message.parts {
                    if let Part::ToolCall(call) = part {
                        out.push_str(&format!("\n[called {} {}]", call.name, call.input));
                    }
                }
            }
            Role::Tool => {
                out.push_str("Tool results:\n");
                for part in &message.parts {
                    if let Part::ToolResult(result) = part {
                        out.push_str(&clip_chars(&result.content, TRANSCRIPT_RESULT_CHARS));
                        out.push('\n');
                    }
                }
            }
        }
        out.push_str("\n\n");
    }
    out.push_str("Write the summary now.");
    out
}

#[derive(Default)]
struct Collect {
    text: String,
    usage: TokenUsage,
}

impl EventSink for Collect {
    fn event(&mut self, event: StreamEvent) {
        match event {
            StreamEvent::Delta(text) => self.text.push_str(&text),
            StreamEvent::Usage(usage) => self.usage = usage,
            // A summary needs no reasoning or tools (none are offered).
            StreamEvent::ReasoningDelta(_)
            | StreamEvent::Reasoning(_)
            | StreamEvent::ToolCall(_) => {}
        }
    }
}
