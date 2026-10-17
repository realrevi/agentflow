//! Line parsers for the CLI session logs. Field names come from each CLI's source code
//! (see `cli/mod.rs` for the files). A line that is not JSON is an `Err` (skipped with a
//! debug log by the caller); valid JSON without usage is `Parsed::Nothing`.

use serde_json::Value;

use crate::providers::TokenUsage;
use crate::tokens::time::parse_iso;

/// What one line contributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parsed {
    /// Usage of one model response.
    Usage(LineUsage),
    /// Session context (Codex `session_meta` / `turn_context`).
    Context {
        cwd: Option<String>,
        model: Option<String>,
    },
    Nothing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineUsage {
    pub id: String,
    pub at: u64,
    pub model: Option<String>,
    pub cwd: Option<String>,
    pub tokens: TokenUsage,
}

fn u64_of(value: &Value) -> u64 {
    value.as_u64().unwrap_or(0)
}

fn text(value: &Value) -> Option<String> {
    value.as_str().filter(|s| !s.is_empty()).map(str::to_owned)
}

fn time(value: &Value) -> u64 {
    value.as_str().and_then(parse_iso).unwrap_or(0)
}

/// Claude Code: `type: "assistant"` lines with `message.{id, model, usage}` (Anthropic usage
/// fields), `requestId`, `cwd`, `timestamp`. One API message spans several lines with the
/// same `message.id` and the same usage: the caller counts each id once.
pub fn claude_line(line: &str) -> Result<Parsed, serde_json::Error> {
    let v: Value = serde_json::from_str(line)?;
    if v["type"] != "assistant" {
        return Ok(Parsed::Nothing);
    }
    let message = &v["message"];
    let usage = &message["usage"];
    let model = text(&message["model"]);
    // `<synthetic>` messages (local errors, interruptions) never reached the API.
    if !usage.is_object() || model.as_deref() == Some("<synthetic>") {
        return Ok(Parsed::Nothing);
    }
    let Some(id) = text(&message["id"]).or_else(|| text(&v["requestId"])) else {
        return Ok(Parsed::Nothing);
    };
    let tokens = TokenUsage {
        input_tokens: u64_of(&usage["input_tokens"]),
        output_tokens: u64_of(&usage["output_tokens"]),
        cache_read_tokens: u64_of(&usage["cache_read_input_tokens"]),
        cache_write_tokens: u64_of(&usage["cache_creation_input_tokens"]),
    };
    if tokens.is_empty() {
        return Ok(Parsed::Nothing);
    }
    Ok(Parsed::Usage(LineUsage {
        id,
        at: time(&v["timestamp"]),
        model,
        cwd: text(&v["cwd"]),
        tokens,
    }))
}

/// Codex `TokenUsage` → ours: there, `input_tokens` include `cached_input_tokens` and
/// `output_tokens` include reasoning.
fn codex_tokens(usage: &Value) -> TokenUsage {
    let input = u64_of(&usage["input_tokens"]);
    let cached = u64_of(&usage["cached_input_tokens"]).min(input);
    TokenUsage {
        input_tokens: input - cached,
        output_tokens: u64_of(&usage["output_tokens"]),
        cache_read_tokens: cached,
        cache_write_tokens: u64_of(&usage["cache_write_input_tokens"]),
    }
}

/// Codex rollout lines `{timestamp, type, payload}`. Newer files have one
/// `token_usage_record` per response (with `response_id`); older ones only `event_msg` /
/// `token_count` with `info.last_token_usage`. `has_records`: the file has records, so the
/// `token_count` events (the same usage again) are ignored.
pub fn codex_line(line: &str, has_records: bool) -> Result<Parsed, serde_json::Error> {
    let v: Value = serde_json::from_str(line)?;
    let payload = &v["payload"];
    let at = time(&v["timestamp"]);
    let usage = |id: String, tokens: TokenUsage| {
        Parsed::Usage(LineUsage {
            id,
            at,
            model: None,
            cwd: None,
            tokens,
        })
    };
    match v["type"].as_str().unwrap_or("") {
        "session_meta" => Ok(Parsed::Context {
            cwd: text(&payload["cwd"]),
            model: None,
        }),
        "turn_context" => Ok(Parsed::Context {
            cwd: text(&payload["cwd"]),
            model: text(&payload["model"]),
        }),
        "token_usage_record" => {
            let tokens = codex_tokens(&payload["usage"]);
            Ok(match text(&payload["response_id"]) {
                Some(id) if !tokens.is_empty() => usage(id, tokens),
                _ => Parsed::Nothing,
            })
        }
        "event_msg" if !has_records && payload["type"] == "token_count" => {
            let info = &payload["info"];
            let tokens = codex_tokens(&info["last_token_usage"]);
            if tokens.is_empty() {
                return Ok(Parsed::Nothing);
            }
            // No response id in old files: the running total identifies the event (Codex
            // repeats a token_count with an unchanged total, which must not count twice).
            let total = u64_of(&info["total_token_usage"]["total_tokens"]);
            Ok(usage(format!("total:{total}"), tokens))
        }
        _ => Ok(Parsed::Nothing),
    }
}

/// One Gemini CLI message record: `{id, timestamp, type: "gemini", model, tokens: {input,
/// output, cached, thoughts, tool, total}}`. `input` (promptTokenCount) includes `cached`;
/// `output` (candidatesTokenCount) leaves out thinking, which is billed as output.
pub fn gemini_message(v: &Value) -> Option<LineUsage> {
    let tokens = &v["tokens"];
    if v["type"] != "gemini" || !tokens.is_object() {
        return None;
    }
    let input = u64_of(&tokens["input"]);
    let cached = u64_of(&tokens["cached"]).min(input);
    let usage = TokenUsage {
        input_tokens: input - cached + u64_of(&tokens["tool"]),
        output_tokens: u64_of(&tokens["output"]) + u64_of(&tokens["thoughts"]),
        cache_read_tokens: cached,
        cache_write_tokens: 0,
    };
    if usage.is_empty() {
        return None;
    }
    Some(LineUsage {
        id: text(&v["id"])?,
        at: time(&v["timestamp"]),
        model: text(&v["model"]),
        cwd: None,
        tokens: usage,
    })
}

/// Gemini session JSONL: message records (re-appended with the same id once their tokens are
/// known) between `$set` / `$patch` / `$rewindTo` records, which carry no usage.
pub fn gemini_line(line: &str) -> Result<Parsed, serde_json::Error> {
    let v: Value = serde_json::from_str(line)?;
    Ok(gemini_message(&v).map_or(Parsed::Nothing, Parsed::Usage))
}
