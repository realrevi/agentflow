//! AI provider entries (phase 5). API keys are never part of settings: they live in the OS
//! keychain (`crate::providers::keys`), keyed by the entry id.

use serde::{Deserialize, Serialize};

/// `anthropic` = native Messages API; `openai` = OpenAI-compatible Chat Completions.
pub const PROVIDER_KINDS: [&str; 2] = ["anthropic", "openai"];
/// Presets of the OpenAI-compatible kind (they prefill the base URL and pick quirks).
pub const OPENAI_PRESETS: [&str; 7] = [
    "openai",
    "deepseek",
    "gemini",
    "openrouter",
    "groq",
    "ollama",
    "custom",
];
pub const ANTHROPIC_PRESET: &str = "anthropic";
pub const MAX_PROVIDERS: usize = 32;
pub const DEFAULT_MAX_TOKENS: u32 = 4096;
pub const MAX_OUTPUT_TOKENS: u32 = 1_000_000;
pub const TEMPERATURE_RANGE: (f64, f64) = (0.0, 2.0);
const MAX_ID_LEN: usize = 64;
const MAX_NAME_LEN: usize = 64;
const MAX_MODEL_LEN: usize = 200;
const MAX_URL_LEN: usize = 2048;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProviderEntry {
    /// Stable id (`[A-Za-z0-9_-]`, at most 64): conversations and the keychain refer to it.
    pub id: String,
    /// One of `PROVIDER_KINDS`.
    pub kind: String,
    /// `anthropic` for the Anthropic kind, else one of `OPENAI_PRESETS`.
    pub preset: String,
    /// Display name; empty = the preset's name.
    pub name: String,
    /// Validated when used (HTTPS only, except localhost).
    pub base_url: String,
    /// Model new chats start with; empty = pick one in the chat.
    pub default_model: String,
    pub max_tokens: u32,
    /// `None` = the provider's default.
    pub temperature: Option<f64>,
}

impl Default for ProviderEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: PROVIDER_KINDS[1].into(),
            preset: "custom".into(),
            name: String::new(),
            base_url: String::new(),
            default_model: String::new(),
            max_tokens: DEFAULT_MAX_TOKENS,
            temperature: None,
        }
    }
}

pub fn is_valid_provider_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}

/// Drops entries with a bad or duplicate id or an unknown kind, and repairs the rest.
pub fn normalize_providers(entries: Vec<ProviderEntry>) -> Vec<ProviderEntry> {
    let mut out: Vec<ProviderEntry> = Vec::new();
    for mut entry in entries {
        if out.len() == MAX_PROVIDERS
            || !is_valid_provider_id(&entry.id)
            || !PROVIDER_KINDS.contains(&entry.kind.as_str())
            || out.iter().any(|other| other.id == entry.id)
        {
            continue;
        }
        if entry.kind == ANTHROPIC_PRESET {
            entry.preset = ANTHROPIC_PRESET.into();
        } else if !OPENAI_PRESETS.contains(&entry.preset.as_str()) {
            entry.preset = "custom".into();
        }
        entry.name = clip(entry.name.trim(), MAX_NAME_LEN);
        entry.base_url = clip(entry.base_url.trim(), MAX_URL_LEN);
        entry.default_model = clip(entry.default_model.trim(), MAX_MODEL_LEN);
        entry.max_tokens = match entry.max_tokens {
            0 => DEFAULT_MAX_TOKENS,
            n => n.min(MAX_OUTPUT_TOKENS),
        };
        let (min, max) = TEMPERATURE_RANGE;
        entry.temperature = entry
            .temperature
            .filter(|t| t.is_finite())
            .map(|t| t.clamp(min, max));
        out.push(entry);
    }
    out
}

/// The first `max` characters (not bytes) of `text`.
fn clip(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, kind: &str, preset: &str) -> ProviderEntry {
        ProviderEntry {
            id: id.into(),
            kind: kind.into(),
            preset: preset.into(),
            ..ProviderEntry::default()
        }
    }

    #[test]
    fn drops_bad_and_duplicate_entries() {
        let entries = vec![
            entry("a", "anthropic", "whatever"),
            entry("a", "openai", "openai"),
            entry("../x", "openai", "openai"),
            entry("b", "cohere", "custom"),
            entry("c", "openai", "nope"),
        ];
        let out = normalize_providers(entries);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].preset, "anthropic");
        assert_eq!(out[1].id, "c");
        assert_eq!(out[1].preset, "custom");
    }

    #[test]
    fn repairs_numbers_and_text() {
        let mut bad = entry("p1", "openai", "ollama");
        bad.max_tokens = 0;
        bad.temperature = Some(f64::NAN);
        bad.name = format!("  {}  ", "n".repeat(100));
        bad.base_url = "  http://localhost:11434/v1  ".into();
        let out = normalize_providers(vec![bad.clone()]);
        assert_eq!(out[0].max_tokens, DEFAULT_MAX_TOKENS);
        assert_eq!(out[0].temperature, None);
        assert_eq!(out[0].name.chars().count(), MAX_NAME_LEN);
        assert_eq!(out[0].base_url, "http://localhost:11434/v1");

        bad.max_tokens = u32::MAX;
        bad.temperature = Some(9.0);
        let out = normalize_providers(vec![bad]);
        assert_eq!(out[0].max_tokens, MAX_OUTPUT_TOKENS);
        assert_eq!(out[0].temperature, Some(2.0));
    }

    #[test]
    fn keeps_at_most_32_entries() {
        let many = (0..40).map(|i| entry(&format!("p{i}"), "openai", "openai"));
        assert_eq!(normalize_providers(many.collect()).len(), MAX_PROVIDERS);
    }
}
