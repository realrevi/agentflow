//! One model call of a turn: build the request, stream it, store the assistant message
//! (parts, usage, prefix digest) and record its usage.

use super::run::Turn;
use crate::chat::{build_with, ChatEvent, RequestOptions, StepSink};
use crate::conversations::{ChatMessage, MessageStatus, Role, UsageRecord};
use crate::providers::{ProviderError, StreamEnd, ToolCall, ToolSpec};
use crate::tokens::chat_record;
use crate::util::{now_ms, run_blocking};

pub struct StepResult {
    pub outcome: Result<StreamEnd, ProviderError>,
    /// The stored assistant message (`None` when nothing arrived).
    pub message: Option<ChatMessage>,
    pub calls: Vec<ToolCall>,
    /// Nothing at all arrived before the outcome (text, reasoning or calls).
    pub empty: bool,
}

/// FNV-1a over the system prompt, tool names and model: stable across runs and versions.
pub fn prefix_digest(system: &str, tools: &[ToolSpec], model: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        hash ^= 0xff;
    };
    feed(system.as_bytes());
    for tool in tools {
        feed(tool.name.as_bytes());
    }
    feed(model.as_bytes());
    format!("{hash:016x}")
}

pub async fn model_call(
    turn: &Turn,
    send: &mut (dyn FnMut(ChatEvent) + Send),
    system: String,
    tools: &[ToolSpec],
) -> Result<StepResult, crate::error::AppError> {
    let conversation = {
        let (store, workspace, id) = (
            turn.store.clone(),
            turn.workspace.clone(),
            turn.conversation_id.clone(),
        );
        run_blocking(move || store.get(workspace.as_deref(), &id)).await?
    };
    let prefix = prefix_digest(&system, tools, &conversation.model);
    let options = RequestOptions {
        caching: turn.settings.chat.prompt_caching && turn.entry.kind == "anthropic",
        system: Some(system),
        tools: tools.to_vec(),
        prefix: Some(prefix.clone()),
        anthropic: turn.entry.kind == "anthropic",
    };
    let chat = build_with(&conversation, &turn.entry, &options)?;
    let mut sink = StepSink::new(send);
    let outcome = turn
        .provider
        .stream_chat(&turn.http, &chat, &mut sink, &turn.cancel)
        .await
        .map_err(|err| err.redacted(turn.secret.as_deref()));
    let usage = (!sink.usage.is_empty()).then(|| UsageRecord {
        provider_id: turn.entry.id.clone(),
        preset: turn.entry.preset.clone(),
        model: chat.model.clone(),
        at: now_ms(),
        tokens: sink.usage,
    });
    let calls = sink.tool_calls();
    let empty = sink.is_empty();
    let (status, stop_reason, error) = match &outcome {
        Ok(StreamEnd::Stopped(reason)) => (MessageStatus::Complete, Some(reason.clone()), None),
        Ok(StreamEnd::Cancelled) => (MessageStatus::Cancelled, Some("cancelled".into()), None),
        Err(err) => (MessageStatus::Error, None, Some(err.message.clone())),
    };
    let keep =
        status == MessageStatus::Complete || !sink.text.trim().is_empty() || !calls.is_empty();
    let message = keep.then(|| ChatMessage {
        id: format!("a{:x}", now_ms()),
        role: Role::Assistant,
        content: std::mem::take(&mut sink.text),
        attachments: Vec::new(),
        at: now_ms(),
        status,
        stop_reason,
        error,
        usage,
        parts: std::mem::take(&mut sink.parts),
        prefix: Some(prefix),
    });
    drop(sink);
    if let Some(message) = &message {
        store_message(turn, message.clone()).await;
        record_usage(turn, message).await;
    }
    Ok(StepResult {
        outcome,
        message,
        calls,
        empty,
    })
}

pub async fn store_message(turn: &Turn, message: ChatMessage) {
    let (store, workspace, id) = (
        turn.store.clone(),
        turn.workspace.clone(),
        turn.conversation_id.clone(),
    );
    if let Err(err) =
        run_blocking(move || store.append_message(workspace.as_deref(), &id, message)).await
    {
        tracing::warn!("could not store an agent step: {err}");
    }
}

/// Every model call's usage goes to the ledger (partial replies too).
async fn record_usage(turn: &Turn, message: &ChatMessage) {
    let Some(usage) = &message.usage else { return };
    let record = chat_record(
        &turn.conversation_id,
        &message.id,
        turn.workspace.as_deref(),
        turn.panel.as_deref(),
        usage,
    );
    let tracker = turn.tracker.clone();
    if let Err(err) = run_blocking(move || tracker.record(vec![record])).await {
        tracing::warn!("could not record usage: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_digest_changes_with_the_prefix_only() {
        let tool = |name: &str| ToolSpec {
            name: name.into(),
            description: String::new(),
            schema: json!({}),
        };
        let base = prefix_digest("sys", &[tool("read_file")], "m");
        assert_eq!(base, prefix_digest("sys", &[tool("read_file")], "m"));
        assert_ne!(base, prefix_digest("sys!", &[tool("read_file")], "m"));
        assert_ne!(
            base,
            prefix_digest("sys", &[tool("read_file"), tool("x")], "m")
        );
        assert_ne!(base, prefix_digest("sys", &[tool("read_file")], "m2"));
        assert_eq!(base.len(), 16);
    }
}
