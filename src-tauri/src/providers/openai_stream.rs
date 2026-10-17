//! OpenAI-compatible SSE decoder: text and refusal deltas, reasoning (`reasoning_content`
//! from DeepSeek/GLM, `reasoning` and `reasoning_details` from OpenRouter), tool calls
//! whose `function.arguments` arrive in fragments keyed by `index`, usage and finish.

use serde_json::{Map, Value};

use super::error::{ErrorKind, ProviderError};
use super::http::{interrupted, stream_error, Decoder, ErrorDetail};
use super::openai::normalize_usage;
use super::sse::SseEvent;
use super::types::{parse_tool_input, EventSink, Reasoning, StreamEvent, ToolCall};

/// Finish reasons that mean the provider gave up rather than finished (DeepSeek).
const FAILED_FINISH: [&str; 2] = ["insufficient_system_resource", "aborted"];

#[derive(Debug, Default)]
struct PendingCall {
    index: u64,
    id: String,
    name: String,
    arguments: String,
}

#[derive(Debug, Default)]
pub struct OpenAiDecoder {
    stop: Option<String>,
    reasoning: String,
    /// OpenRouter `reasoning_details`, merged by `index` (fragments of one item append).
    details: Vec<Value>,
    calls: Vec<PendingCall>,
    flushed: bool,
}

impl OpenAiDecoder {
    fn tool_call_delta(&mut self, item: &Value) {
        let id = item["id"].as_str().filter(|id| !id.is_empty());
        let index = match item["index"].as_u64() {
            Some(index) => index,
            // No index (some servers send whole calls): a new id is a new call.
            None => match id {
                Some(id) if !self.calls.iter().any(|c| c.id == id) => self.calls.len() as u64,
                _ => self.calls.last().map_or(0, |c| c.index),
            },
        };
        let position = match self.calls.iter().position(|c| c.index == index) {
            Some(position) => position,
            None => {
                self.calls.push(PendingCall {
                    index,
                    ..PendingCall::default()
                });
                self.calls.len() - 1
            }
        };
        let call = &mut self.calls[position];
        if let Some(id) = id {
            call.id = id.to_owned();
        }
        let function = &item["function"];
        if let Some(name) = function["name"].as_str().filter(|n| !n.is_empty()) {
            call.name = name.to_owned();
        }
        match &function["arguments"] {
            Value::String(fragment) => call.arguments.push_str(fragment),
            // A few servers send the arguments as an object.
            Value::Object(_) => call.arguments = function["arguments"].to_string(),
            _ => {}
        }
    }

    fn reasoning_details(&mut self, items: &[Value]) {
        for item in items {
            let index = item["index"].as_u64();
            let existing = self
                .details
                .iter_mut()
                .find(|d| index.is_some() && d["index"].as_u64() == index);
            match (existing, item) {
                (Some(Value::Object(into)), Value::Object(from)) => merge_detail(into, from),
                _ => self.details.push(item.clone()),
            }
        }
    }

    /// Reasoning first, then the tool calls in index order; sent once per reply.
    fn flush_parts(&mut self, sink: &mut dyn EventSink) {
        if self.flushed {
            return;
        }
        self.flushed = true;
        if !self.reasoning.is_empty() || !self.details.is_empty() {
            let text = if self.reasoning.is_empty() {
                self.details
                    .iter()
                    .filter_map(|d| d["text"].as_str())
                    .collect::<String>()
            } else {
                std::mem::take(&mut self.reasoning)
            };
            let details =
                (!self.details.is_empty()).then(|| Value::Array(std::mem::take(&mut self.details)));
            sink.event(StreamEvent::Reasoning(Reasoning {
                text,
                details,
                ..Reasoning::default()
            }));
        }
        self.calls.sort_by_key(|c| c.index);
        for call in std::mem::take(&mut self.calls) {
            let id = if call.id.is_empty() {
                format!("call_{}", call.index)
            } else {
                call.id
            };
            sink.event(StreamEvent::ToolCall(ToolCall {
                id,
                name: call.name,
                input: parse_tool_input(&call.arguments),
            }));
        }
    }
}

/// Appends streamed string fields (`text`, `summary`, `data`, `signature`); other fields
/// keep their first value.
fn merge_detail(into: &mut Map<String, Value>, from: &Map<String, Value>) {
    for (key, value) in from {
        match (into.get_mut(key), value) {
            (Some(Value::String(existing)), Value::String(more))
                if matches!(key.as_str(), "text" | "summary" | "data" | "signature") =>
            {
                existing.push_str(more);
            }
            (None, _) => {
                into.insert(key.clone(), value.clone());
            }
            _ => {}
        }
    }
}

impl Decoder for OpenAiDecoder {
    fn event(
        &mut self,
        event: &SseEvent,
        sink: &mut dyn EventSink,
    ) -> Result<Option<String>, ProviderError> {
        let data = event.data.trim();
        if data == "[DONE]" {
            self.flush_parts(sink);
            return Ok(Some(self.stop.take().unwrap_or_else(|| "stop".into())));
        }
        let Ok(chunk) = serde_json::from_str::<Value>(data) else {
            return Ok(None);
        };
        // In-band errors: {"error":{...}} (OpenAI, OpenRouter), [{"error":{...}}] (Google)
        // and Groq's `x_groq.error` string.
        if let Some(detail) = ErrorDetail::from_json(&chunk) {
            return Err(stream_error(&detail));
        }
        if let Some(message) = chunk["x_groq"]["error"].as_str() {
            return Err(stream_error(&ErrorDetail {
                message: message.to_owned(),
                ..ErrorDetail::default()
            }));
        }
        // Groq used to report streamed usage under `x_groq`.
        let usage = chunk
            .get("usage")
            .filter(|u| u.is_object())
            .or_else(|| chunk["x_groq"].get("usage").filter(|u| u.is_object()));
        if let Some(usage) = usage {
            sink.event(StreamEvent::Usage(normalize_usage(usage)));
        }
        let Some(choice) = chunk["choices"].as_array().and_then(|c| c.first()) else {
            return Ok(None); // usage-only or debug chunk
        };
        let delta = &choice["delta"];
        for field in ["reasoning_content", "reasoning"] {
            if let Some(text) = delta[field].as_str().filter(|t| !t.is_empty()) {
                self.reasoning.push_str(text);
                sink.event(StreamEvent::ReasoningDelta(text.to_owned()));
            }
        }
        if let Some(items) = delta["reasoning_details"].as_array() {
            self.reasoning_details(items);
        }
        // A refusal is shown as the reply.
        for field in ["content", "refusal"] {
            if let Some(text) = delta[field].as_str().filter(|t| !t.is_empty()) {
                sink.event(StreamEvent::Delta(text.to_owned()));
            }
        }
        for item in delta["tool_calls"].as_array().into_iter().flatten() {
            self.tool_call_delta(item);
        }
        if let Some(reason) = choice["finish_reason"].as_str() {
            if FAILED_FINISH.contains(&reason) {
                return Err(ProviderError::new(
                    ErrorKind::Server,
                    format!("The provider stopped the reply ({reason})."),
                ));
            }
            self.stop = Some(reason.to_owned());
        }
        Ok(None)
    }

    /// Some servers close without `[DONE]`; a reply that already finished still counts.
    fn finish(&mut self, sink: &mut dyn EventSink) -> Result<String, ProviderError> {
        let stop = self.stop.take().ok_or_else(interrupted)?;
        self.flush_parts(sink);
        Ok(stop)
    }
}
