//! Anthropic Messages request bodies: turns as content blocks (text, thinking replayed
//! unchanged, tool_use, tool_result), tools, the system prompt and cache breakpoints.

use serde_json::{json, Value};

use super::types::{ChatRequest, ChatRole, ChatTurn, Part, TurnBlock};

/// Prompt-cache breakpoint (5-minute TTL). The API allows at most 4 per request; the chat
/// request builder places them (see `chat/cache.rs`).
fn cache_control() -> Value {
    json!({ "type": "ephemeral" })
}

/// One content block; `None` for blocks that cannot be sent (empty text, thinking without
/// its signature).
fn block(turn_block: &TurnBlock) -> Option<Value> {
    let mut block = match &turn_block.part {
        Part::Text { text } if text.is_empty() => return None,
        Part::Text { text } => json!({ "type": "text", "text": text }),
        Part::Reasoning(reasoning) => match (&reasoning.redacted, &reasoning.signature) {
            (Some(data), _) => json!({ "type": "redacted_thinking", "data": data }),
            (None, Some(signature)) => json!({
                "type": "thinking",
                "thinking": reasoning.text,
                "signature": signature,
            }),
            (None, None) => return None,
        },
        Part::ToolCall(call) => json!({
            "type": "tool_use",
            "id": call.id,
            "name": call.name,
            "input": call.input,
        }),
        Part::ToolResult(result) => {
            let mut block = json!({
                "type": "tool_result",
                "tool_use_id": result.call_id,
                "content": result.content,
            });
            if result.is_error {
                block["is_error"] = json!(true);
            }
            block
        }
    };
    if turn_block.cache {
        block["cache_control"] = cache_control();
    }
    Some(block)
}

/// A plain-text turn without breakpoints is a string; anything else is a list of blocks.
fn content(turn: &ChatTurn) -> Value {
    let plain = turn
        .blocks
        .iter()
        .all(|b| !b.cache && matches!(b.part, Part::Text { .. }));
    if plain {
        return json!(turn.content());
    }
    json!(turn.blocks.iter().filter_map(block).collect::<Vec<_>>())
}

pub fn request_body(chat: &ChatRequest) -> Value {
    let messages: Vec<Value> = chat
        .turns
        .iter()
        .map(|turn| {
            let role = match turn.role {
                ChatRole::User => "user",
                ChatRole::Assistant => "assistant",
            };
            json!({ "role": role, "content": content(turn) })
        })
        .collect();
    let mut body = json!({
        "model": chat.model,
        "max_tokens": chat.max_tokens,
        "messages": messages,
        "stream": true,
    });
    if !chat.tools.is_empty() {
        let tools: Vec<Value> = chat
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.schema,
                })
            })
            .collect();
        body["tools"] = json!(tools);
    }
    if !chat.system.is_empty() {
        body["system"] = if chat.caching {
            json!([{ "type": "text", "text": chat.system, "cache_control": cache_control() }])
        } else {
            json!(chat.system)
        };
    }
    if let Some(temperature) = chat.temperature {
        body["temperature"] = json!(temperature);
    }
    body
}
