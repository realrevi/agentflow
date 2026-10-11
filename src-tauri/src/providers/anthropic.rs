//! Anthropic Messages API (native): POST {base}/v1/messages with SSE streaming, and
//! GET {base}/v1/models (paginated). Headers: x-api-key, anthropic-version 2023-06-01.

use reqwest::Url;
use serde_json::{json, Value};

use super::cancel::CancelToken;
use super::error::ProviderError;
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

pub const API_VERSION: &str = "2023-06-01";
const MODELS_PAGE: u32 = 1000;
const MAX_MODEL_PAGES: usize = 10;

pub struct Anthropic {
    base: Url,
    key: Option<ApiKey>,
}

impl Anthropic {
    pub fn new(base: Url, key: Option<ApiKey>) -> Self {
        Self { base, key }
    }

    /// Accepts a base with or without the `/v1` suffix.
    fn url(&self, path: &str) -> Url {
        if self.base.path().trim_end_matches('/').ends_with("/v1") {
            endpoint(&self.base, path)
        } else {
            endpoint(&self.base, &format!("v1/{path}"))
        }
    }

    fn authorize(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, ProviderError> {
        let request = request.header("anthropic-version", API_VERSION);
        match &self.key {
            Some(key) => Ok(request.header("x-api-key", secret_header(key.expose())?)),
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
            .authorize(http.post(self.url("messages")))?
            .header("accept", "text/event-stream")
            .header("content-type", "application/json")
            .body(request_body(chat).to_string());
        let Some(response) = send(request, Some(cancel)).await? else {
            return Ok(StreamEnd::Cancelled);
        };
        pump(response, &mut AnthropicDecoder::default(), sink, cancel).await
    }

    async fn models(&self, http: &Http) -> Result<Vec<ModelInfo>, ProviderError> {
        let mut models = Vec::new();
        let mut after: Option<String> = None;
        for _ in 0..MAX_MODEL_PAGES {
            let mut url = self.url("models");
            url.query_pairs_mut()
                .append_pair("limit", &MODELS_PAGE.to_string());
            if let Some(after) = &after {
                url.query_pairs_mut().append_pair("after_id", after);
            }
            let request = self.authorize(http.get(url))?.timeout(SHORT_TIMEOUT);
            let Some(response) = send(request, None).await? else {
                break;
            };
            let page = read_json(response).await?;
            for model in page["data"].as_array().into_iter().flatten() {
                if let Some(id) = model["id"].as_str() {
                    models.push(ModelInfo {
                        id: id.to_owned(),
                        name: model["display_name"].as_str().map(str::to_owned),
                    });
                }
            }
            match (page["has_more"].as_bool(), page["last_id"].as_str()) {
                (Some(true), Some(last)) => after = Some(last.to_owned()),
                _ => break,
            }
        }
        Ok(models)
    }
}

impl Provider for Anthropic {
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

pub fn request_body(chat: &ChatRequest) -> Value {
    let messages: Vec<Value> = chat
        .turns
        .iter()
        .map(|turn| {
            let role = match turn.role {
                ChatRole::User => "user",
                ChatRole::Assistant => "assistant",
            };
            json!({ "role": role, "content": turn.content })
        })
        .collect();
    let mut body = json!({
        "model": chat.model,
        "max_tokens": chat.max_tokens,
        "messages": messages,
        "stream": true,
    });
    if !chat.system.is_empty() {
        body["system"] = json!(chat.system);
    }
    if let Some(temperature) = chat.temperature {
        body["temperature"] = json!(temperature);
    }
    body
}

/// Usage merges as the official SDK does: `message_start` gives the first counts, every
/// `message_delta` replaces `output_tokens` and any other field it carries (they are
/// cumulative, not increments).
#[derive(Debug, Default)]
pub struct AnthropicDecoder {
    usage: TokenUsage,
    stop: Option<String>,
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
        match kind {
            "message_start" => {
                self.merge_usage(&data["message"]["usage"]);
                sink.event(StreamEvent::Usage(self.usage));
            }
            "content_block_delta" => {
                // Only text deltas: thinking, signature, tool-input and citation deltas
                // are not part of a plain chat reply.
                if data["delta"]["type"] == "text_delta" {
                    if let Some(text) = data["delta"]["text"].as_str().filter(|t| !t.is_empty()) {
                        sink.event(StreamEvent::Delta(text.to_owned()));
                    }
                }
            }
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
            _ => {} // ping, content_block_start/stop and future event types
        }
        Ok(None)
    }

    fn finish(&mut self) -> Result<String, ProviderError> {
        Err(interrupted()) // a complete reply always ends with message_stop
    }
}
