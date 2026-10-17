//! Anthropic Messages API (native): POST {base}/v1/messages with SSE streaming, and
//! GET {base}/v1/models (paginated). Headers: x-api-key, anthropic-version 2023-06-01.

use reqwest::Url;

use super::cancel::CancelToken;
use super::error::ProviderError;
use super::http::{pump, read_json, secret_header, send, Http, SHORT_TIMEOUT};
use super::keys::ApiKey;
use super::provider::{BoxFuture, Provider};
use super::types::{ChatRequest, EventSink, ModelInfo, StreamEnd};
use super::url::endpoint;

pub use super::anthropic_request::request_body;
pub use super::anthropic_stream::AnthropicDecoder;

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
                    let mut info =
                        ModelInfo::named(id, model["display_name"].as_str().map(str::to_owned));
                    info.context_window = model["max_input_tokens"].as_u64();
                    models.push(info);
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
