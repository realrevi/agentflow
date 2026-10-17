//! OpenAI-compatible Chat Completions request bodies: messages (tool calls on the assistant,
//! results as `tool` messages, the reasoning echo the preset needs), tools as functions,
//! usage options and the max-tokens field per preset.

use serde_json::{json, Value};

use super::openai::Preset;
use super::types::{ChatRequest, ChatRole, ChatTurn, Part};

pub fn request_body(chat: &ChatRequest, preset: Preset) -> Value {
    let mut messages = Vec::with_capacity(chat.turns.len() + 1);
    if !chat.system.is_empty() {
        messages.push(json!({ "role": preset.system_role(), "content": chat.system }));
    }
    for turn in &chat.turns {
        match turn.role {
            ChatRole::User => user_messages(turn, &mut messages),
            ChatRole::Assistant => messages.push(assistant_message(turn, preset)),
        }
    }
    let mut body = json!({
        "model": chat.model,
        "messages": messages,
        "stream": true,
    });
    if preset.wants_stream_options() {
        // Usage in the stream: a final chunk (OpenAI, Ollama, most local servers) or on
        // the finish chunk (DeepSeek); Gemini sends it on every chunk anyway.
        body["stream_options"] = json!({ "include_usage": true });
    }
    body[preset.max_tokens_field()] = json!(chat.max_tokens);
    if let Some(temperature) = chat.temperature {
        body["temperature"] = json!(temperature);
    }
    if preset == Preset::OpenAi {
        // Drops the random `obfuscation` padding OpenAI adds to every chunk.
        body["stream_options"]["include_obfuscation"] = json!(false);
    }
    if !chat.tools.is_empty() {
        let tools: Vec<Value> = chat
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.schema,
                    },
                })
            })
            .collect();
        body["tools"] = json!(tools);
    }
    body
}

/// Tool results become `tool` messages (they must follow the assistant's calls); the rest
/// of the turn is one user message.
fn user_messages(turn: &ChatTurn, messages: &mut Vec<Value>) {
    let mut text = String::new();
    for block in &turn.blocks {
        match &block.part {
            Part::ToolResult(result) => {
                let content = if result.is_error {
                    format!("Error: {}", result.content)
                } else {
                    result.content.clone()
                };
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": result.call_id,
                    "content": content,
                }));
            }
            Part::Text { text: piece } => text.push_str(piece),
            Part::Reasoning(_) | Part::ToolCall(_) => {}
        }
    }
    if !text.is_empty() {
        messages.push(json!({ "role": "user", "content": text }));
    }
}

/// Text as `content` (null when the message only calls tools), calls as `tool_calls` with
/// their arguments as a JSON string, and the reasoning echo the preset needs.
fn assistant_message(turn: &ChatTurn, preset: Preset) -> Value {
    let text = turn.content();
    let calls: Vec<Value> = turn
        .blocks
        .iter()
        .filter_map(|block| match &block.part {
            Part::ToolCall(call) => Some(json!({
                "id": call.id,
                "type": "function",
                "function": { "name": call.name, "arguments": call.input.to_string() },
            })),
            _ => None,
        })
        .collect();
    let mut message = json!({ "role": "assistant", "content": text });
    if !calls.is_empty() {
        if text.is_empty() {
            message["content"] = Value::Null;
        }
        message["tool_calls"] = json!(calls);
    }
    let reasoning: Vec<&super::types::Reasoning> = turn
        .blocks
        .iter()
        .filter_map(|block| match &block.part {
            Part::Reasoning(reasoning) => Some(reasoning),
            _ => None,
        })
        .collect();
    match preset.reasoning_echo() {
        Some(field) if !reasoning.is_empty() => {
            let details: Vec<Value> = reasoning
                .iter()
                .filter_map(|r| r.details.as_ref())
                .flat_map(|d| d.as_array().cloned().unwrap_or_default())
                .collect();
            if preset == Preset::OpenRouter && !details.is_empty() {
                message["reasoning_details"] = json!(details);
            } else {
                let text: String = reasoning.iter().map(|r| r.text.as_str()).collect();
                if !text.is_empty() {
                    message[field] = json!(text);
                }
            }
        }
        _ => {}
    }
    message
}
