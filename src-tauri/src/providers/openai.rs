//! OpenAI-compatible Chat Completions: POST {base}/chat/completions with SSE streaming and
//! GET {base}/models. One adapter for OpenAI, DeepSeek, Gemini (OpenAI layer), OpenRouter,
//! Groq, Ollama and custom servers; presets only switch documented quirks.

use reqwest::Url;
use serde_json::{json, Value};

use super::cancel::CancelToken;
use super::error::{ErrorKind, ProviderError};
use super::http::{
    interrupted, pump, read_json, secret_header, send, stream_error, Decoder, ErrorDetail, Http,
    SHORT_TIMEOUT,
};
use super::keys::ApiKey;
use super::provider::{BoxFuture, Provider};
use super::sse::SseEvent;
use super::types::{
    ChatRequest, ChatRole, EventSink, ModelInfo, StreamEnd, StreamEvent, TokenUsage,
};
use super::url::endpoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    OpenAi,
    DeepSeek,
    Gemini,
    OpenRouter,
    Groq,
    Ollama,
    Custom,
}

impl Preset {
    pub fn parse(name: &str) -> Self {
        match name {
            "openai" => Self::OpenAi,
            "deepseek" => Self::DeepSeek,
            "gemini" => Self::Gemini,
            "openrouter" => Self::OpenRouter,
            "groq" => Self::Groq,
            "ollama" => Self::Ollama,
            _ => Self::Custom,
        }
    }

    /// OpenAI's reasoning models take instructions as `developer`; DeepSeek rejects that
    /// role and everyone else documents `system`.
    fn system_role(self) -> &'static str {
        match self {
            Self::OpenAi => "developer",
            _ => "system",
        }
    }

    /// OpenAI deprecated `max_tokens` (reasoning models reject it); OpenRouter and Groq
    /// document `max_completion_tokens` too. DeepSeek, Gemini, Ollama and most local
    /// servers know `max_tokens` only.
    fn max_tokens_field(self) -> &'static str {
        match self {
            Self::OpenAi | Self::OpenRouter | Self::Groq => "max_completion_tokens",
            _ => "max_tokens",
        }
    }

    /// Usage is requested with `stream_options.include_usage` where that is documented.
    /// Groq's request schema rejects unknown fields (and sends usage anyway); OpenRouter
    /// always sends usage and calls the option deprecated.
    fn wants_stream_options(self) -> bool {
        !matches!(self, Self::Groq | Self::OpenRouter)
    }

    /// Gemini's OpenAI layer lists ids as `models/<id>` but takes bare ids in requests.
    fn model_prefix(self) -> Option<&'static str> {
        (self == Self::Gemini).then_some("models/")
    }
}

pub struct OpenAiCompatible {
    base: Url,
    key: Option<ApiKey>,
    preset: Preset,
}

impl OpenAiCompatible {
    pub fn new(base: Url, key: Option<ApiKey>, preset: Preset) -> Self {
        Self { base, key, preset }
    }

    fn authorize(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, ProviderError> {
        let request = match self.preset {
            // Optional app attribution (https://openrouter.ai/docs/app-attribution).
            Preset::OpenRouter => request
                .header("HTTP-Referer", "https://github.com/realrevi/agentflow")
                .header("X-OpenRouter-Title", "AgentFlow"),
            _ => request,
        };
        match &self.key {
            Some(key) => Ok(request.header(
                "authorization",
                secret_header(&format!("Bearer {}", key.expose()))?,
            )),
            None => Ok(request),
        }
    }

    async fn stream(
        &self,
        http: &Http,
        chat: &ChatRequest,
        sink: &mut dyn EventSink,
        cancel: &CancelToken,
    ) -> Result<StreamEnd, ProviderError> {
        let request = self
            .authorize(http.post(endpoint(&self.base, "chat/completions")))?
            .header("accept", "text/event-stream")
            .header("content-type", "application/json")
            .body(request_body(chat, self.preset).to_string());
        let Some(response) = send(request, Some(cancel)).await? else {
            return Ok(StreamEnd::Cancelled);
        };
        pump(response, &mut OpenAiDecoder::default(), sink, cancel).await
    }

    async fn models(&self, http: &Http) -> Result<Vec<ModelInfo>, ProviderError> {
        let request = self
            .authorize(http.get(endpoint(&self.base, "models")))?
            .timeout(SHORT_TIMEOUT);
        let Some(response) = send(request, None).await? else {
            return Ok(Vec::new());
        };
        Ok(parse_models(&read_json(response).await?, self.preset))
    }
}

impl Provider for OpenAiCompatible {
    fn stream_chat<'a>(
        &'a self,
        http: &'a Http,
        chat: &'a ChatRequest,
        sink: &'a mut dyn EventSink,
        cancel: &'a CancelToken,
    ) -> BoxFuture<'a, Result<StreamEnd, ProviderError>> {
        Box::pin(self.stream(http, chat, sink, cancel))
    }

    fn list_models<'a>(
        &'a self,
        http: &'a Http,
    ) -> BoxFuture<'a, Result<Vec<ModelInfo>, ProviderError>> {
        Box::pin(self.models(http))
    }
}

pub fn request_body(chat: &ChatRequest, preset: Preset) -> Value {
    let mut messages = Vec::with_capacity(chat.turns.len() + 1);
    if !chat.system.is_empty() {
        messages.push(json!({ "role": preset.system_role(), "content": chat.system }));
    }
    for turn in &chat.turns {
        let role = match turn.role {
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
        };
        messages.push(json!({ "role": role, "content": turn.content }));
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
    body
}

pub fn parse_models(list: &Value, preset: Preset) -> Vec<ModelInfo> {
    let mut models: Vec<ModelInfo> = list["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|model| {
            let raw = model["id"].as_str()?;
            let id = preset
                .model_prefix()
                .and_then(|prefix| raw.strip_prefix(prefix))
                .unwrap_or(raw);
            Some(ModelInfo {
                id: id.to_owned(),
                name: model["name"]
                    .as_str()
                    .filter(|n| *n != id)
                    .map(str::to_owned),
            })
        })
        .collect();
    models.sort_by(|a, b| a.id.cmp(&b.id));
    models.dedup_by(|a, b| a.id == b.id);
    models
}

/// Finish reasons that mean the provider gave up rather than finished (DeepSeek).
const FAILED_FINISH: [&str; 2] = ["insufficient_system_resource", "aborted"];

#[derive(Debug, Default)]
pub struct OpenAiDecoder {
    stop: Option<String>,
}

impl Decoder for OpenAiDecoder {
    fn event(
        &mut self,
        event: &SseEvent,
        sink: &mut dyn EventSink,
    ) -> Result<Option<String>, ProviderError> {
        let data = event.data.trim();
        if data == "[DONE]" {
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
        // Reasoning text (`reasoning_content`, `reasoning`) and tool calls are not part
        // of a plain chat reply; a refusal is shown as the reply.
        for field in ["content", "refusal"] {
            if let Some(text) = delta[field].as_str().filter(|t| !t.is_empty()) {
                sink.event(StreamEvent::Delta(text.to_owned()));
            }
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
    fn finish(&mut self) -> Result<String, ProviderError> {
        self.stop.take().ok_or_else(interrupted)
    }
}

/// OpenAI-style usage, normalized: `prompt_tokens` includes cache reads (OpenAI,
/// OpenRouter, DeepSeek) and Gemini leaves thinking out of `completion_tokens` but in
/// `total_tokens`, so output = max(completion, total - prompt).
pub fn normalize_usage(usage: &Value) -> TokenUsage {
    let n = |value: &Value| value.as_u64().unwrap_or(0);
    let prompt = n(&usage["prompt_tokens"]);
    let completion = n(&usage["completion_tokens"]);
    let total = n(&usage["total_tokens"]);
    let details = &usage["prompt_tokens_details"];
    let cached = n(&details["cached_tokens"]).max(n(&usage["prompt_cache_hit_tokens"]));
    TokenUsage {
        input_tokens: prompt.saturating_sub(cached),
        output_tokens: completion.max(total.saturating_sub(prompt)),
        cache_read_tokens: cached,
        cache_write_tokens: n(&details["cache_write_tokens"]),
    }
}
