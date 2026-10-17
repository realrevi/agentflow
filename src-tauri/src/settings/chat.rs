//! API chat options (phase 5) and smart token management (phase 6): prompt caching, context
//! windows and compaction.

use serde::{Deserialize, Serialize};

use super::providers::is_valid_provider_id;

pub const DEFAULT_CONTEXT_WINDOW: u32 = 128_000;
pub const CONTEXT_WINDOW_RANGE: (u32, u32) = (1_000, 10_000_000);
pub const COMPACT_THRESHOLD_RANGE: (f64, f64) = (0.3, 0.95);
pub const COMPACT_KEEP_RANGE: (u32, u32) = (1, 50);
const MAX_CONTEXT_WINDOWS: usize = 200;
const MAX_MODEL_LEN: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChatSettings {
    /// Streamed text is revealed smoothly; off = shown as it arrives.
    pub smooth_reveal: bool,
    /// Anthropic: mark the system prompt, attachments and the stable prefix for caching.
    pub prompt_caching: bool,
    /// Used when neither the models endpoint nor `context_windows` knows the model.
    pub default_context_window: u32,
    /// Per-model context windows set by the user.
    pub context_windows: Vec<ModelWindow>,
    /// Share of the context window at which the prompt bar warns (and auto-compact runs).
    pub compact_threshold: f64,
    /// Compact keeps this many recent turns (a prompt and its reply) verbatim.
    pub compact_keep_turns: u32,
    /// The (cheaper) model that writes summaries; `None` = the chat's own provider.
    pub compact_provider_id: Option<String>,
    /// Empty = the chat's own model.
    pub compact_model: String,
    pub auto_compact: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelWindow {
    pub model: String,
    pub tokens: u32,
}

impl Default for ModelWindow {
    fn default() -> Self {
        Self {
            model: String::new(),
            tokens: DEFAULT_CONTEXT_WINDOW,
        }
    }
}

impl Default for ChatSettings {
    fn default() -> Self {
        Self {
            smooth_reveal: true,
            prompt_caching: true,
            default_context_window: DEFAULT_CONTEXT_WINDOW,
            context_windows: Vec::new(),
            compact_threshold: 0.7,
            compact_keep_turns: 4,
            compact_provider_id: None,
            compact_model: String::new(),
            auto_compact: false,
        }
    }
}

impl ChatSettings {
    pub fn normalized(mut self) -> Self {
        let (min, max) = CONTEXT_WINDOW_RANGE;
        self.default_context_window = self.default_context_window.clamp(min, max);
        let mut windows: Vec<ModelWindow> = Vec::new();
        for mut window in std::mem::take(&mut self.context_windows) {
            window.model = clip(window.model.trim(), MAX_MODEL_LEN);
            if window.model.is_empty() || windows.iter().any(|w| w.model == window.model) {
                continue;
            }
            window.tokens = window.tokens.clamp(min, max);
            windows.push(window);
        }
        windows.truncate(MAX_CONTEXT_WINDOWS);
        self.context_windows = windows;
        let (low, high) = COMPACT_THRESHOLD_RANGE;
        self.compact_threshold = if self.compact_threshold.is_finite() {
            self.compact_threshold.clamp(low, high)
        } else {
            0.7
        };
        let (fewest, most) = COMPACT_KEEP_RANGE;
        self.compact_keep_turns = self.compact_keep_turns.clamp(fewest, most);
        self.compact_provider_id = self
            .compact_provider_id
            .take()
            .filter(|id| is_valid_provider_id(id));
        self.compact_model = clip(self.compact_model.trim(), MAX_MODEL_LEN);
        self
    }

    /// The user's window for `model`, if set.
    pub fn window_for(&self, model: &str) -> Option<u32> {
        self.context_windows
            .iter()
            .find(|w| w.model == model)
            .map(|w| w.tokens)
    }
}

pub(super) fn clip(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_out_of_range_values() {
        let chat = ChatSettings {
            default_context_window: 5,
            context_windows: vec![
                ModelWindow {
                    model: " m1 ".into(),
                    tokens: u32::MAX,
                },
                ModelWindow {
                    model: "m1".into(),
                    tokens: 9000,
                },
                ModelWindow {
                    model: "  ".into(),
                    tokens: 9000,
                },
            ],
            compact_threshold: f64::NAN,
            compact_keep_turns: 0,
            compact_provider_id: Some("bad id".into()),
            ..ChatSettings::default()
        }
        .normalized();
        assert_eq!(chat.default_context_window, 1_000);
        assert_eq!(chat.context_windows.len(), 1);
        assert_eq!(chat.window_for("m1"), Some(10_000_000));
        assert_eq!(chat.compact_threshold, 0.7);
        assert_eq!(chat.compact_keep_turns, 1);
        assert_eq!(chat.compact_provider_id, None);
    }

    #[test]
    fn defaults_are_kept() {
        assert_eq!(
            ChatSettings::default().normalized(),
            ChatSettings::default()
        );
    }
}
