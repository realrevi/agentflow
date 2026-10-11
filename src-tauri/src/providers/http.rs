//! The one HTTP client (rustls with the ring provider, no redirects, generous read timeout
//! for slow reasoning models) and the parts every adapter shares: turning error responses
//! into `ProviderError`s and pumping an SSE body into a decoder until it ends or is cancelled.

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{RequestBuilder, Response, StatusCode};
use serde_json::Value;

use super::cancel::CancelToken;
use super::error::{parse_retry_after, ErrorKind, ProviderError};
use super::sse::{SseEvent, SseParser};
use super::types::{EventSink, StreamEnd};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Between two reads of a streaming body. Reasoning models can think for minutes; servers
/// that queue requests send keep-alive comments, which count as reads.
const READ_TIMEOUT: Duration = Duration::from_secs(300);
/// Whole request, for small calls such as listing models.
pub const SHORT_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_ERROR_BODY: usize = 64 * 1024;
const MAX_JSON_BODY: usize = 16 * 1024 * 1024;
const MAX_MESSAGE_CHARS: usize = 400;

/// Managed state: cheap to clone (reqwest pools connections inside).
#[derive(Clone)]
pub struct Http {
    client: reqwest::Client,
}

impl Http {
    pub fn new() -> Result<Self, String> {
        Self::build(READ_TIMEOUT, false)
    }

    /// Tests: short timeouts and no system proxy (they talk to a local mock server).
    #[cfg(test)]
    pub fn for_tests(read_timeout: Duration) -> Self {
        Self::build(read_timeout, true).expect("test client")
    }

    fn build(read_timeout: Duration, no_proxy: bool) -> Result<Self, String> {
        // reqwest is built without a bundled crypto provider; ring is ours. Installing twice
        // is harmless (the second call just reports the first one).
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut builder = reqwest::Client::builder()
            .user_agent(concat!("AgentFlow/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(read_timeout)
            // A redirect could carry the key header to another host: never follow one.
            .redirect(reqwest::redirect::Policy::none());
        if no_proxy {
            builder = builder.no_proxy();
        }
        builder
            .build()
            .map(|client| Self { client })
            .map_err(|err| format!("could not create the HTTP client: {err}"))
    }

    pub fn post(&self, url: reqwest::Url) -> RequestBuilder {
        self.client.post(url)
    }

    pub fn get(&self, url: reqwest::Url) -> RequestBuilder {
        self.client.get(url)
    }
}

/// A header value that hyper/reqwest will never print (keys, tokens).
pub fn secret_header(value: &str) -> Result<HeaderValue, ProviderError> {
    let mut header = HeaderValue::from_str(value).map_err(|_| {
        ProviderError::config("The API key contains characters a header cannot hold.")
    })?;
    header.set_sensitive(true);
    Ok(header)
}

/// Sends a request; a cancel before the response arrives ends it right away.
pub async fn send(
    request: RequestBuilder,
    cancel: Option<&CancelToken>,
) -> Result<Option<Response>, ProviderError> {
    let response = match cancel {
        Some(cancel) => tokio::select! {
            biased;
            () = cancel.cancelled() => return Ok(None),
            response = request.send() => response,
        },
        None => request.send().await,
    };
    let response = response.map_err(transport_error)?;
    if response.status().is_success() {
        return Ok(Some(response));
    }
    Err(error_response(response).await)
}

/// Reads a whole (non-streaming) JSON body, e.g. a model list.
pub async fn read_json(mut response: Response) -> Result<Value, ProviderError> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        body.extend_from_slice(&chunk);
        if body.len() > MAX_JSON_BODY {
            return Err(ProviderError::new(
                ErrorKind::Server,
                "The provider's answer is too large.",
            ));
        }
    }
    serde_json::from_slice(&body)
        .map_err(|_| ProviderError::new(ErrorKind::Server, "The provider sent invalid JSON."))
}

/// Reads an SSE body into `decoder` until it reports the end, the body ends, or the user
/// cancels. Deltas of one network chunk are flushed to the sink together.
pub async fn pump(
    mut response: Response,
    decoder: &mut dyn Decoder,
    sink: &mut dyn EventSink,
    cancel: &CancelToken,
) -> Result<StreamEnd, ProviderError> {
    let mut parser = SseParser::new();
    loop {
        let chunk = tokio::select! {
            biased;
            () = cancel.cancelled() => return Ok(StreamEnd::Cancelled),
            chunk = response.chunk() => chunk.map_err(transport_error)?,
        };
        let Some(bytes) = chunk else {
            let last = parser.finish();
            let done = match last {
                Some(event) => decoder.event(&event, sink)?,
                None => None,
            };
            sink.flush();
            return match done {
                Some(stop) => Ok(StreamEnd::Stopped(stop)),
                None => decoder.finish().map(StreamEnd::Stopped),
            };
        };
        let events = parser.push(&bytes).map_err(|_| {
            ProviderError::new(ErrorKind::Server, "The provider sent a malformed stream.")
        })?;
        for event in events {
            if let Some(stop) = decoder.event(&event, sink)? {
                sink.flush();
                return Ok(StreamEnd::Stopped(stop));
            }
        }
        sink.flush();
    }
}

/// Turns one provider's SSE events into stream events.
pub trait Decoder: Send {
    /// Handles one event; `Some(stop_reason)` once the reply is complete.
    fn event(
        &mut self,
        event: &SseEvent,
        sink: &mut dyn EventSink,
    ) -> Result<Option<String>, ProviderError>;

    /// The body ended without the provider's end marker: `Ok(stop)` if the reply was in
    /// fact complete, else an error.
    fn finish(&mut self) -> Result<String, ProviderError>;
}

pub fn interrupted() -> ProviderError {
    ProviderError::new(
        ErrorKind::Network,
        "The connection closed before the reply was complete.",
    )
}

pub fn transport_error(err: reqwest::Error) -> ProviderError {
    let kind = if err.is_timeout() {
        ErrorKind::Timeout
    } else {
        ErrorKind::Network
    };
    let message = if err.is_timeout() {
        "The provider did not answer in time.".to_owned()
    } else if err.is_connect() {
        format!("Could not connect to the provider ({}).", root_cause(&err))
    } else {
        format!(
            "The connection to the provider failed ({}).",
            root_cause(&err)
        )
    };
    ProviderError::new(kind, message)
}

/// The innermost error message (e.g. "Connection refused"), without the URL.
fn root_cause(err: &(dyn std::error::Error + 'static)) -> String {
    let mut current = err;
    while let Some(next) = current.source() {
        current = next;
    }
    current.to_string()
}

async fn error_response(response: Response) -> ProviderError {
    let status = response.status();
    let headers = response.headers().clone();
    let mut body = Vec::new();
    let mut response = response;
    while let Ok(Some(chunk)) = response.chunk().await {
        body.extend_from_slice(&chunk);
        if body.len() >= MAX_ERROR_BODY {
            break;
        }
    }
    http_error(status, &headers, &body)
}

/// Classifies a non-2xx answer. Bodies come in many shapes: `{"error":{...}}` (Anthropic,
/// OpenAI and most compatibles), `[{"error":{...}}]` (Gemini), `{"error":"text"}` or text.
pub fn http_error(status: StatusCode, headers: &HeaderMap, body: &[u8]) -> ProviderError {
    let parsed = serde_json::from_slice::<Value>(body).ok();
    let detail = parsed
        .as_ref()
        .and_then(ErrorDetail::from_json)
        .unwrap_or_else(|| ErrorDetail {
            message: String::from_utf8_lossy(body).trim().to_owned(),
            ..ErrorDetail::default()
        });
    let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
    let retry_after = parse_retry_after(header("retry-after"), header("retry-after-ms"));
    let kind = classify(status, &detail);
    let reason = status.canonical_reason().unwrap_or("error");
    let mut message = if detail.message.is_empty() {
        format!("HTTP {} {reason}", status.as_u16())
    } else {
        format!("HTTP {}: {}", status.as_u16(), detail.message)
    };
    if message.chars().count() > MAX_MESSAGE_CHARS {
        message = format!(
            "{}…",
            message.chars().take(MAX_MESSAGE_CHARS).collect::<String>()
        );
    }
    ProviderError {
        kind,
        message,
        status: Some(status.as_u16()),
        retry_after: retry_after
            .filter(|_| kind == ErrorKind::RateLimit || kind == ErrorKind::Overloaded),
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ErrorDetail {
    pub message: String,
    /// `type` (Anthropic/OpenAI), `status` (Google) or `metadata.error_type` (OpenRouter).
    pub kind: String,
    pub code: String,
    /// A numeric `code` is an HTTP status (OpenRouter, Google).
    pub status: Option<u16>,
}

impl ErrorDetail {
    /// Reads `{"error": ...}` or `[{"error": ...}]`; `None` if neither is there.
    pub fn from_json(value: &Value) -> Option<Self> {
        let error = match value {
            Value::Array(items) => items.first()?.get("error")?,
            other => other.get("error")?,
        };
        let text = |key: &str| match error.get(key) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            _ => String::new(),
        };
        Some(match error {
            Value::String(message) => Self {
                message: message.clone(),
                ..Self::default()
            },
            _ => {
                let kind = [text("type"), text("status")]
                    .into_iter()
                    .chain(error["metadata"]["error_type"].as_str().map(str::to_owned))
                    .find(|k| !k.is_empty())
                    .unwrap_or_default();
                Self {
                    message: text("message"),
                    kind,
                    code: text("code"),
                    status: error["code"].as_u64().and_then(|c| u16::try_from(c).ok()),
                }
            }
        })
    }
}

fn classify(status: StatusCode, detail: &ErrorDetail) -> ErrorKind {
    let text = format!("{} {} {}", detail.kind, detail.code, detail.message).to_ascii_lowercase();
    let about_model = text.contains("model_not_found")
        || text.contains("model_decommissioned")
        || text.contains("model not exist")
        || (text.contains("model")
            && (text.contains("not found") || text.contains("does not exist")));
    let about_key = text.contains("api key")
        || text.contains("api_key")
        || text.contains("authorization header")
        || text.contains("authentication");
    match status.as_u16() {
        401 | 403 => ErrorKind::Auth,
        404 => ErrorKind::InvalidModel,
        408 | 504 => ErrorKind::Timeout,
        429 => ErrorKind::RateLimit,
        503 | 529 => ErrorKind::Overloaded,
        400 | 422 if about_key => ErrorKind::Auth,
        400 | 422 if about_model => ErrorKind::InvalidModel,
        500..=599 => ErrorKind::Server,
        _ => ErrorKind::BadRequest,
    }
}

/// Classifies an error object that arrived inside a 200 stream.
pub fn stream_error(detail: &ErrorDetail) -> ProviderError {
    let text = format!("{} {}", detail.kind, detail.code).to_ascii_lowercase();
    let by_status = detail.status.and_then(|status| {
        StatusCode::from_u16(status)
            .ok()
            .filter(|s| s.is_client_error() || s.is_server_error())
            .map(|s| classify(s, detail))
    });
    let kind = if let Some(kind) = by_status {
        kind
    } else if text.contains("overloaded") || text.contains("unavailable") {
        ErrorKind::Overloaded
    } else if text.contains("rate_limit") || text.contains("resource_exhausted") {
        ErrorKind::RateLimit
    } else if text.contains("authentication") || text.contains("permission") {
        ErrorKind::Auth
    } else if text.contains("invalid_request") || text.contains("invalid_argument") {
        ErrorKind::BadRequest
    } else {
        ErrorKind::Server
    };
    let message = if detail.message.is_empty() {
        "The provider reported an error while streaming.".to_owned()
    } else {
        detail.message.clone()
    };
    ProviderError::new(kind, message)
}
