//! One line of the usage ledger: who used how many tokens, when and where.

use serde::{Deserialize, Serialize};

use crate::providers::TokenUsage;

/// Where the usage came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    Api,
    ClaudeCode,
    Codex,
    GeminiCli,
}

impl Source {
    pub fn is_cli(self) -> bool {
        self != Self::Api
    }

    /// Budget/provider key of CLI usage (API usage uses its provider entry id).
    pub fn key(self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::ClaudeCode => "claude-code",
            Self::Codex => "codex",
            Self::GeminiCli => "gemini-cli",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRecord {
    /// ms since the Unix epoch.
    pub at: u64,
    pub source: Source,
    /// API: the provider entry id; CLI: the source key (`claude-code`...).
    pub provider: String,
    /// API: the preset (`anthropic`, `openrouter`...), for pricing; CLI: the vendor.
    #[serde(default)]
    pub preset: String,
    pub model: String,
    /// The workspace folder, if known.
    #[serde(default)]
    pub workspace: Option<String>,
    /// The AgentFlow panel, if known.
    #[serde(default)]
    pub panel: Option<String>,
    /// Dedup key (message/request id); empty = none.
    #[serde(default)]
    pub id: String,
    /// `compact` for summaries written by Compact; empty for normal replies.
    #[serde(default)]
    pub kind: String,
    #[serde(flatten)]
    pub tokens: TokenUsage,
}

impl UsageRecord {
    pub fn total_tokens(&self) -> u64 {
        let t = &self.tokens;
        t.input_tokens + t.output_tokens + t.cache_read_tokens + t.cache_write_tokens
    }
}

#[cfg(test)]
pub fn sample(at: u64, source: Source, model: &str, input: u64, output: u64) -> UsageRecord {
    UsageRecord {
        at,
        source,
        provider: if source == Source::Api {
            "p1".into()
        } else {
            source.key().into()
        },
        preset: "anthropic".into(),
        model: model.into(),
        workspace: Some("/work/demo".into()),
        panel: None,
        id: String::new(),
        kind: String::new(),
        tokens: TokenUsage {
            input_tokens: input,
            output_tokens: output,
            ..TokenUsage::default()
        },
    }
}
