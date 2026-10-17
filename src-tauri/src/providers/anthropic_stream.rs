//! Anthropic Messages SSE decoder: text, thinking (with its signature), redacted thinking
//! and tool_use blocks whose input streams as `input_json_delta` fragments.

use std::collections::HashMap;

use serde_json::Value;

use super::error::ProviderError;
use super::http::{interrupted, stream_error, Decoder, ErrorDetail};
use super::sse::SseEvent;
use super::types::{parse_tool_input, EventSink, Reasoning, StreamEvent, TokenUsage, ToolCall};

/// A content block being streamed.
#[derive(Debug)]
enum Open {
    Text,
    Thinking {
        text: String,
        signature: Option<String>,
    },
    Redacted(String),
    ToolUse {
        id: String,
        name: String,
        json: String,
    },
}

/// Usage merges as the official SDK does: `message_start` gives the first counts, every
/// `message_delta` replaces `output_tokens` and any other field it carries (they are
/// cumulative, not increments). Tool inputs arrive as `input_json_delta` fragments and are
/// parsed at `content_block_stop`; thinking keeps its `signature` for the replay.
#[derive(Debug, Default)]
pub struct AnthropicDecoder {
    usage: TokenUsage,
    stop: Option<String>,
    blocks: HashMap<u64, Open>,
}

impl AnthropicDecoder {
    fn merge_usage(&mut self, usage: &Value) {
        let field = |name: &str| usage.get(name).and_then(Value::as_u64);
        if let Some(n) = field("input_tokens") {
            self.usage.input_tokens = n;
        }
        if let Some(n) = field("output_tokens") {
            self.usage.output_tokens = n;
        }
        if let Some(n) = field("cache_read_input_tokens") {
            self.usage.cache_read_tokens = n;
        }
        if let Some(n) = field("cache_creation_input_tokens") {
            self.usage.cache_write_tokens = n;
        }
    }

    fn start_block(&mut self, index: u64, block: &Value, sink: &mut dyn EventSink) {
        let text = |key: &str| block[key].as_str().unwrap_or("").to_owned();
        let open = match block["type"].as_str().unwrap_or("") {
            "text" => {
                let initial = text("text");
                if !initial.is_empty() {
                    sink.event(StreamEvent::Delta(initial));
                }
                Open::Text
            }
            "thinking" => Open::Thinking {
                text: text("thinking"),
                signature: block["signature"].as_str().map(str::to_owned),
            },
            "redacted_thinking" => Open::Redacted(text("data")),
            "tool_use" => Open::ToolUse {
                id: text("id"),
                name: text("name"),
                json: String::new(),
            },
            _ => return, // server tools, citations... are not ours
        };
        self.blocks.insert(index, open);
    }

    fn delta(&mut self, index: u64, delta: &Value, sink: &mut dyn EventSink) {
        let piece = |key: &str| delta[key].as_str().unwrap_or("");
        match (
            delta["type"].as_str().unwrap_or(""),
            self.blocks.get_mut(&index),
        ) {
            ("text_delta", _) => {
                if !piece("text").is_empty() {
                    sink.event(StreamEvent::Delta(piece("text").to_owned()));
                }
            }
            ("thinking_delta", Some(Open::Thinking { text, .. })) => {
                text.push_str(piece("thinking"));
                if !piece("thinking").is_empty() {
                    sink.event(StreamEvent::ReasoningDelta(piece("thinking").to_owned()));
                }
            }
            ("signature_delta", Some(Open::Thinking { signature, .. })) => {
                signature
                    .get_or_insert_with(String::new)
                    .push_str(piece("signature"));
            }
            ("input_json_delta", Some(Open::ToolUse { json, .. })) => {
                json.push_str(piece("partial_json"));
            }
            _ => {}
        }
    }

    fn stop_block(&mut self, index: u64, sink: &mut dyn EventSink) {
        match self.blocks.remove(&index) {
            Some(Open::Thinking { text, signature }) => {
                sink.event(StreamEvent::Reasoning(Reasoning {
                    text,
                    signature,
                    ..Reasoning::default()
                }));
            }
            Some(Open::Redacted(data)) => sink.event(StreamEvent::Reasoning(Reasoning {
                redacted: Some(data),
                ..Reasoning::default()
            })),
            Some(Open::ToolUse { id, name, json }) => {
                sink.event(StreamEvent::ToolCall(ToolCall {
                    id,
                    name,
                    input: parse_tool_input(&json),
                }));
            }
            Some(Open::Text) | None => {}
        }
    }
}

impl Decoder for AnthropicDecoder {
    fn event(
        &mut self,
        event: &SseEvent,
        sink: &mut dyn EventSink,
    ) -> Result<Option<String>, ProviderError> {
        let Ok(data) = serde_json::from_str::<Value>(&event.data) else {
            return Ok(None); // not JSON: nothing we know
        };
        let kind = data["type"]
            .as_str()
            .or(event.event.as_deref())
            .unwrap_or("");
        let index = data["index"].as_u64().unwrap_or(0);
        match kind {
            "message_start" => {
                self.merge_usage(&data["message"]["usage"]);
                sink.event(StreamEvent::Usage(self.usage));
            }
            "content_block_start" => self.start_block(index, &data["content_block"], sink),
            "content_block_delta" => self.delta(index, &data["delta"], sink),
            "content_block_stop" => self.stop_block(index, sink),
            "message_delta" => {
                if let Some(reason) = data["delta"]["stop_reason"].as_str() {
                    self.stop = Some(reason.to_owned());
                }
                self.merge_usage(&data["usage"]);
                sink.event(StreamEvent::Usage(self.usage));
            }
            "message_stop" => {
                return Ok(Some(self.stop.take().unwrap_or_else(|| "end_turn".into())));
            }
            "error" => {
                let detail = ErrorDetail::from_json(&data).unwrap_or_default();
                return Err(stream_error(&detail));
            }
            _ => {} // ping and future event types
        }
        Ok(None)
    }

    fn finish(&mut self, _sink: &mut dyn EventSink) -> Result<String, ProviderError> {
        Err(interrupted()) // a complete reply always ends with message_stop
    }
}
