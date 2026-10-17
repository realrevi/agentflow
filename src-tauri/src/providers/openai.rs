//! OpenAI-compatible Chat Completions: POST {base}/chat/completions with SSE streaming and
//! GET {base}/models. One adapter for OpenAI, DeepSeek, Gemini (OpenAI layer), OpenRouter,
//! Groq, Ollama and custom servers; presets only switch documented quirks.

use reqwest::Url;
use serde_json::Value;

use super::cancel::CancelToken;
use super::error::ProviderError;
use super::http::{pump, read_json, secret_header, send, Http, SHORT_TIMEOUT};
use super::keys::ApiKey;
use super::provider::{BoxFuture, Provider};
use super::types::{ChatRequest, EventSink, ModelInfo, StreamEnd, TokenUsage};
use super::url::endpoint;

pub use super::openai_request::request_body;
pub use super::openai_stream::OpenAiDecoder;

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
    pub(super) fn system_role(self) -> &'static str {
        match self {
            Self::OpenAi => "developer",
            _ => "system",
        }
    }

    /// OpenAI deprecated `max_tokens` (reasoning models reject it); OpenRouter and Groq
    /// document `max_completion_tokens` too. DeepSeek, Gemini, Ollama and most local
    /// servers know `max_tokens` only.
    pub(super) fn max_tokens_field(self) -> &'static str {
        match self {
            Self::OpenAi | Self::OpenRouter | Self::Groq => "max_completion_tokens",
            _ => "max_tokens",
        }
    }

    /// Usage is requested with `stream_options.include_usage` where that is documented.
    /// Groq's request schema rejects unknown fields (and sends usage anyway); OpenRouter
    /// always sends usage and calls the option deprecated.
    pub(super) fn wants_stream_options(self) -> bool {
        !matches!(self, Self::Groq | Self::OpenRouter)
    }

    /// Gemini's OpenAI layer lists ids as `models/<id>` but takes bare ids in requests.
    fn model_prefix(self) -> Option<&'static str> {
        (self == Self::Gemini).then_some("models/")
    }

    /// How earlier reasoning goes back on assistant messages. DeepSeek (and GLM-style
    /// custom servers) require `reasoning_content` once tools are in the request (else a
    /// 400); OpenRouter wants `reasoning_details` unchanged (or `reasoning`). OpenAI, Gemini,
    /// Groq and Ollama document no such field, so nothing is sent there.
    pub(super) fn reasoning_echo(self) -> Option<&'static str> {
        match self {
            Self::DeepSeek | Self::Custom => Some("reasoning_content"),
            Self::OpenRouter => Some("reasoning"),
            Self::OpenAi | Self::Gemini | Self::Groq | Self::Ollama => None,
        }
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

pub fn parse_models(list: &Value, preset: Preset) -> Vec<ModelInfo> {
    let mut models: Vec<ModelInfo> = list["data"]
        .as_array()
        .into_iter()
        .flatten()
        // Groq marks retired or unavailable models with `active: false`.
        .filter(|model| model["active"].as_bool() != Some(false))
        .filter_map(|model| {
            let raw = model["id"].as_str()?;
            let id = preset
                .model_prefix()
                .and_then(|prefix| raw.strip_prefix(prefix))
                .unwrap_or(raw);
            let name = model["name"]
                .as_str()
                .filter(|n| *n != id)
                .map(str::to_owned);
            let mut info = ModelInfo::named(id, name);
            // OpenRouter lists `context_length` and per-token `pricing`.
            info.context_window = model["context_length"].as_u64();
            info.pricing = model.get("pricing").filter(|p| p.is_object()).cloned();
            Some(info)
        })
        .collect();
    models.sort_by(|a, b| a.id.cmp(&b.id));
    models.dedup_by(|a, b| a.id == b.id);
    models
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
