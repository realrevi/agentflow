//! Errors of a provider request, safe to show: every message is redacted before it leaves
//! the providers module, so an API key can never reach the UI or the logs.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    /// 401/403: missing, wrong or revoked key, or no access.
    Auth,
    /// 429: too many requests or tokens (see `retry_after`), or a spent quota.
    RateLimit,
    /// The model does not exist or this key cannot use it.
    InvalidModel,
    /// Other 4xx: the provider rejected the request (parameters, size, billing...).
    BadRequest,
    /// 529/503: the provider is overloaded.
    Overloaded,
    /// Other 5xx, or an error event in the middle of the stream.
    Server,
    /// Could not connect, or the connection broke.
    Network,
    /// No answer within the time limit.
    Timeout,
    /// Our side: no key, a bad base URL, an unknown provider entry.
    Config,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderError {
    pub kind: ErrorKind,
    pub message: String,
    /// HTTP status, when the provider answered with one.
    pub status: Option<u16>,
    /// Seconds to wait before retrying, when the provider said (429/503).
    pub retry_after: Option<u64>,
}

impl ProviderError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            status: None,
            retry_after: None,
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Config, message)
    }

    /// Removes `secret` (and anything that looks like an API key) from the message.
    pub fn redacted(mut self, secret: Option<&str>) -> Self {
        self.message = redact(&self.message, secret);
        self
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ProviderError {}

const MASK: &str = "[redacted]";
/// Prefixes of well-known API key formats (OpenAI, Anthropic, Groq, OpenRouter, Google...).
const KEY_PREFIXES: [&str; 6] = ["sk-", "gsk_", "AIza", "xai-", "pk-", "hf_"];
const MIN_KEY_TAIL: usize = 12;

/// Replaces `secret` wherever it appears, and any token that starts like an API key, with
/// `[redacted]`. Providers sometimes echo a (partial) key back in error messages.
pub fn redact(text: &str, secret: Option<&str>) -> String {
    let mut out = match secret.map(str::trim).filter(|s| s.len() >= 4) {
        Some(secret) => text.replace(secret, MASK),
        None => text.to_owned(),
    };
    for prefix in KEY_PREFIXES {
        out = mask_prefixed(&out, prefix);
    }
    out
}

fn mask_prefixed(text: &str, prefix: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(prefix) {
        let (before, from) = rest.split_at(at);
        out.push_str(before);
        let starts_word = before
            .chars()
            .last()
            .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        let tail: &str = {
            let after = &from[prefix.len()..];
            let end = after
                .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '*')))
                .unwrap_or(after.len());
            after[..end].trim_end_matches('.') // a sentence's full stop is not part of the key
        };
        let tail_len = tail.len();
        if starts_word && tail_len >= MIN_KEY_TAIL {
            out.push_str(MASK);
            rest = &from[prefix.len() + tail_len..];
        } else {
            out.push_str(prefix);
            rest = &from[prefix.len()..];
        }
    }
    out.push_str(rest);
    out
}

/// `Retry-After` is seconds or an HTTP date; `retry-after-ms` (non-standard) wins when sent.
pub fn parse_retry_after(seconds: Option<&str>, millis: Option<&str>) -> Option<u64> {
    if let Some(ms) = millis.and_then(|v| v.trim().parse::<f64>().ok()) {
        if ms.is_finite() && ms >= 0.0 {
            return Some((ms / 1000.0).ceil() as u64);
        }
    }
    let value = seconds?.trim();
    if let Ok(secs) = value.parse::<f64>() {
        return (secs.is_finite() && secs >= 0.0).then(|| secs.ceil() as u64);
    }
    http_date_from_now(value)
}

/// Seconds from now until an IMF-fixdate such as `Wed, 21 Oct 2026 07:28:00 GMT`.
fn http_date_from_now(value: &str) -> Option<u64> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let [_, day, month, year, time, "GMT"] = parts.as_slice() else {
        return None;
    };
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month = MONTHS.iter().position(|m| m == month)? as i64 + 1;
    let (day, year): (i64, i64) = (day.parse().ok()?, year.parse().ok()?);
    let hms: Vec<i64> = time.split(':').filter_map(|p| p.parse().ok()).collect();
    let [h, m, s] = hms.as_slice() else {
        return None;
    };
    let target = days_from_civil(year, month, day) * 86_400 + h * 3600 + m * 60 + s;
    let now = i64::try_from(crate::util::now_ms() / 1000).ok()?;
    u64::try_from(target - now).ok().or(Some(0))
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_the_key_and_key_like_tokens() {
        let key = "sk-ant-api03-abcdefghijklmnopqrstuvwxyz";
        let text = format!("invalid x-api-key {key} (sent as Bearer {key})");
        let out = redact(&text, Some(key));
        assert!(!out.contains("abcdefghij"), "{out}");
        assert_eq!(out.matches(MASK).count(), 2);

        // Providers echo partial keys; the shape alone is enough to mask them.
        let echoed = "Incorrect API key provided: sk-proj-AbCd1234****wxyz. See docs.";
        assert_eq!(
            redact(echoed, None),
            "Incorrect API key provided: [redacted]. See docs."
        );
        assert!(!redact("key=AIzaSyA1234567890abcdefgh", None).contains("AIzaSy"));
        assert!(!redact("gsk_0123456789abcdefABCDEF", None).contains("0123456789"));
    }

    #[test]
    fn leaves_ordinary_text_alone() {
        let text = "task-1 risk-free sk-short model gpt-sk-12345678901234";
        assert_eq!(redact(text, None), text);
        assert_eq!(redact("no secret here", Some("")), "no secret here");
        assert_eq!(
            redact("hello", Some("x")),
            "hello",
            "tiny secrets are ignored"
        );
    }

    #[test]
    fn redacted_errors_keep_their_kind() {
        let err =
            ProviderError::new(ErrorKind::Auth, "bad key sk-1234567890abcdefghij").redacted(None);
        assert_eq!(err.kind, ErrorKind::Auth);
        assert_eq!(err.message, "bad key [redacted]");
    }

    #[test]
    fn parses_retry_after() {
        assert_eq!(parse_retry_after(Some("30"), None), Some(30));
        assert_eq!(parse_retry_after(Some("1.2"), None), Some(2));
        assert_eq!(parse_retry_after(Some("30"), Some("1500")), Some(2));
        assert_eq!(parse_retry_after(Some("soon"), None), None);
        assert_eq!(parse_retry_after(None, None), None);
        assert_eq!(
            parse_retry_after(Some("Thu, 01 Jan 1970 00:00:00 GMT"), None),
            Some(0),
            "a date in the past means retry now"
        );
        let future = parse_retry_after(Some("Fri, 01 Jan 2100 00:00:00 GMT"), None);
        assert!(future.is_some_and(|s| s > 1_000_000));
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
    }
}
