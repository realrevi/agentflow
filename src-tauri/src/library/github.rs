//! GitHub access for the library: the REST API (repository, branch head, one recursive tree
//! per source) and raw.githubusercontent.com for file contents. Every API answer updates the
//! known rate limit; once it is used up, no request is sent until it resets (and a 403/429
//! with no requests left says when). The optional token goes to the API host only.

use std::sync::Mutex;
use std::time::Duration;

use reqwest::header::{HeaderMap, ACCEPT, AUTHORIZATION, ETAG, IF_NONE_MATCH};
use reqwest::{StatusCode, Url};
use serde_json::Value;

use super::model::Rate;
use crate::providers::{secret_header, Http};
use crate::util::{lock, now_ms};

const TIMEOUT: Duration = Duration::from_secs(30);
/// API answers (a recursive tree of a large repository is a few MB).
const MAX_JSON: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Endpoints {
    pub api: String,
    pub raw: String,
    /// The newest `registry/sources.json` of AgentFlow itself.
    pub registry: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            api: "https://api.github.com".into(),
            raw: "https://raw.githubusercontent.com".into(),
            registry:
                "https://raw.githubusercontent.com/realrevi/agentflow/main/registry/sources.json"
                    .into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GhError {
    /// No requests left until `reset_at` (ms since the epoch).
    RateLimited {
        reset_at: u64,
    },
    NotFound,
    TooLarge,
    Network(String),
    Http(u16),
    Invalid(String),
}

impl GhError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::RateLimited { .. } => "rateLimited",
            Self::NotFound => "notFound",
            Self::TooLarge => "tooLarge",
            Self::Network(_) => "network",
            Self::Http(_) => "http",
            Self::Invalid(_) => "invalid",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::RateLimited { .. } => "GitHub's rate limit is used up".into(),
            Self::NotFound => "not found on GitHub (or private)".into(),
            Self::TooLarge => "the file is too large".into(),
            Self::Network(detail) => format!("could not reach GitHub ({detail})"),
            Self::Http(status) => format!("GitHub answered HTTP {status}"),
            Self::Invalid(detail) => detail.clone(),
        }
    }
}

pub enum Fetched<T> {
    Fresh { body: T, etag: Option<String> },
    NotModified,
}

pub struct GitHub {
    http: Http,
    endpoints: Endpoints,
    token: Option<String>,
    rate: Mutex<Option<Rate>>,
}

impl GitHub {
    pub fn new(
        http: Http,
        endpoints: Endpoints,
        token: Option<String>,
        rate: Option<Rate>,
    ) -> Self {
        Self {
            http,
            endpoints,
            token,
            rate: Mutex::new(rate),
        }
    }

    pub fn rate(&self) -> Option<Rate> {
        *lock(&self.rate)
    }

    /// Refuses at once while a used-up limit has not reset yet (never hammer the API).
    fn check_rate(&self) -> Result<(), GhError> {
        match self.rate() {
            Some(rate) if rate.remaining == 0 && rate.reset_at > now_ms() => {
                Err(GhError::RateLimited {
                    reset_at: rate.reset_at,
                })
            }
            _ => Ok(()),
        }
    }

    fn note_rate(&self, headers: &HeaderMap) {
        let number = |name: &str| {
            headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok())
        };
        if let (Some(remaining), Some(reset)) =
            (number("x-ratelimit-remaining"), number("x-ratelimit-reset"))
        {
            *lock(&self.rate) = Some(Rate {
                limit: number("x-ratelimit-limit").unwrap_or(0),
                remaining,
                reset_at: reset.saturating_mul(1000),
            });
        }
    }

    /// A 403/429 that means "slow down": no requests left, or a `retry-after`.
    fn limited(&self, status: StatusCode, headers: &HeaderMap) -> Option<GhError> {
        if status != StatusCode::FORBIDDEN && status != StatusCode::TOO_MANY_REQUESTS {
            return None;
        }
        let retry = headers
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<u64>().ok());
        let rate = self.rate().filter(|r| r.remaining == 0);
        let reset_at = match (retry, rate) {
            (Some(secs), _) => now_ms() + secs * 1000,
            (None, Some(rate)) => rate.reset_at,
            (None, None) if status == StatusCode::TOO_MANY_REQUESTS => now_ms() + 60_000,
            (None, None) => return None,
        };
        let mut known = lock(&self.rate);
        let limit = known.map_or(0, |r| r.limit);
        *known = Some(Rate {
            limit,
            remaining: 0,
            reset_at,
        });
        Some(GhError::RateLimited { reset_at })
    }

    fn url(base: &str, path: &str) -> Result<Url, GhError> {
        Url::parse(&format!(
            "{}/{}",
            base.trim_end_matches('/'),
            path.trim_start_matches('/')
        ))
        .map_err(|_| GhError::Invalid(format!("bad URL path {path}")))
    }

    /// A REST API call (`repos/...`) with an optional ETag.
    pub async fn api(&self, path: &str, etag: Option<&str>) -> Result<Fetched<Value>, GhError> {
        self.check_rate()?;
        let mut request = self
            .http
            .get(Self::url(&self.endpoints.api, path)?)
            .timeout(TIMEOUT)
            .header(ACCEPT, "application/vnd.github+json")
            .header("x-github-api-version", "2022-11-28");
        if let Some(token) = &self.token {
            let value = secret_header(&format!("Bearer {token}"))
                .map_err(|_| GhError::Invalid("the GitHub token is not valid text".into()))?;
            request = request.header(AUTHORIZATION, value);
        }
        if let Some(etag) = etag {
            request = request.header(IF_NONE_MATCH, etag);
        }
        let response = request.send().await.map_err(network)?;
        let (status, headers) = (response.status(), response.headers().clone());
        self.note_rate(&headers);
        if status == StatusCode::NOT_MODIFIED {
            return Ok(Fetched::NotModified);
        }
        if let Some(limited) = self.limited(status, &headers) {
            return Err(limited);
        }
        if status == StatusCode::NOT_FOUND {
            return Err(GhError::NotFound);
        }
        if !status.is_success() {
            return Err(GhError::Http(status.as_u16()));
        }
        let etag = etag_of(&headers);
        let bytes = read_capped(response, MAX_JSON).await?;
        let body = serde_json::from_slice(&bytes)
            .map_err(|_| GhError::Invalid("GitHub sent invalid JSON".into()))?;
        Ok(Fetched::Fresh { body, etag })
    }

    /// A file at a commit from raw.githubusercontent.com (no API quota; it may 429).
    pub async fn raw(
        &self,
        repo: &str,
        commit: &str,
        path: &str,
        cap: usize,
    ) -> Result<Vec<u8>, GhError> {
        let encoded: Vec<String> = path.split('/').map(encode_segment).collect();
        let url = Self::url(
            &self.endpoints.raw,
            &format!("{repo}/{commit}/{}", encoded.join("/")),
        )?;
        self.plain(url, None, cap)
            .await
            .and_then(|fetched| match fetched {
                Fetched::Fresh { body, .. } => Ok(body),
                Fetched::NotModified => Err(GhError::Invalid("unexpected 304".into())),
            })
    }

    /// AgentFlow's own registry file, with its ETag.
    pub async fn registry(&self, etag: Option<&str>) -> Result<Fetched<Vec<u8>>, GhError> {
        let url = Url::parse(&self.endpoints.registry)
            .map_err(|_| GhError::Invalid("bad registry URL".into()))?;
        self.plain(url, etag, 1024 * 1024).await
    }

    async fn plain(
        &self,
        url: Url,
        etag: Option<&str>,
        cap: usize,
    ) -> Result<Fetched<Vec<u8>>, GhError> {
        let mut request = self.http.get(url).timeout(TIMEOUT);
        if let Some(etag) = etag {
            request = request.header(IF_NONE_MATCH, etag);
        }
        let response = request.send().await.map_err(network)?;
        let status = response.status();
        if status == StatusCode::NOT_MODIFIED {
            return Ok(Fetched::NotModified);
        }
        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok())
                .unwrap_or(60);
            return Err(GhError::RateLimited {
                reset_at: now_ms() + retry * 1000,
            });
        }
        if status == StatusCode::NOT_FOUND {
            return Err(GhError::NotFound);
        }
        if !status.is_success() {
            return Err(GhError::Http(status.as_u16()));
        }
        let etag = etag_of(response.headers());
        let body = read_capped(response, cap).await?;
        Ok(Fetched::Fresh { body, etag })
    }
}

fn etag_of(headers: &HeaderMap) -> Option<String> {
    headers
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

fn network(err: reqwest::Error) -> GhError {
    let detail = if err.is_timeout() {
        "timed out".to_owned()
    } else if err.is_connect() {
        "no connection".to_owned()
    } else {
        "connection failed".to_owned()
    };
    GhError::Network(detail)
}

async fn read_capped(mut response: reqwest::Response, cap: usize) -> Result<Vec<u8>, GhError> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(network)? {
        body.extend_from_slice(&chunk);
        if body.len() > cap {
            return Err(GhError::TooLarge);
        }
    }
    Ok(body)
}

/// Percent-encodes one path segment (spaces, `#`, `?`, `%` and non-ASCII).
fn encode_segment(segment: &str) -> String {
    let mut out = String::new();
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
