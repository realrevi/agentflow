//! API agent options (phase 7): loop limits, tool output and command limits, the default
//! permission mode, the default system prompt and per-model "supports tools" overrides.

use serde::{Deserialize, Serialize};

use super::chat::clip;
use crate::conversations::PermissionMode;

pub const MAX_ITERATIONS_RANGE: (u32, u32) = (1, 200);
pub const RESULT_CAP_RANGE: (u32, u32) = (1_000, 1_000_000);
pub const TIMEOUT_RANGE: (u32, u32) = (5, 3_600);
const MAX_MODEL_OVERRIDES: usize = 200;
const MAX_MODEL_LEN: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AgentSettings {
    /// Model calls per user prompt before the turn stops.
    pub max_iterations: u32,
    /// Tool output sent to the model is cut at this many bytes.
    pub tool_result_cap: u32,
    /// `run_command` timeout (seconds) when the model gives none.
    pub command_timeout_secs: u32,
    pub default_mode: PermissionMode,
    /// Prepend AgentFlow's system prompt (identity, environment, tools, rules).
    pub default_prompt: bool,
    /// Show the model's reasoning ("Thinking") in chats; it is stored either way.
    pub show_reasoning: bool,
    /// The user's "supports tools" overrides; models not listed default to on (and are
    /// turned off automatically when the provider rejects tools).
    pub tool_models: Vec<ModelTools>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelTools {
    pub model: String,
    pub tools: bool,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            max_iterations: 25,
            tool_result_cap: 20_000,
            command_timeout_secs: 120,
            default_mode: PermissionMode::Ask,
            default_prompt: true,
            show_reasoning: true,
            tool_models: Vec::new(),
        }
    }
}

impl AgentSettings {
    pub fn normalized(mut self) -> Self {
        let clamp = |value: u32, (min, max): (u32, u32)| value.clamp(min, max);
        self.max_iterations = clamp(self.max_iterations, MAX_ITERATIONS_RANGE);
        self.tool_result_cap = clamp(self.tool_result_cap, RESULT_CAP_RANGE);
        self.command_timeout_secs = clamp(self.command_timeout_secs, TIMEOUT_RANGE);
        let mut models: Vec<ModelTools> = Vec::new();
        for mut entry in std::mem::take(&mut self.tool_models) {
            entry.model = clip(entry.model.trim(), MAX_MODEL_LEN);
            if !entry.model.is_empty() && !models.iter().any(|m| m.model == entry.model) {
                models.push(entry);
            }
        }
        models.truncate(MAX_MODEL_OVERRIDES);
        self.tool_models = models;
        self
    }

    /// The user's override for `model`, if any.
    pub fn tools_override(&self, model: &str) -> Option<bool> {
        self.tool_models
            .iter()
            .find(|m| m.model == model)
            .map(|m| m.tools)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_out_of_range_values_and_duplicate_models() {
        let agent = AgentSettings {
            max_iterations: 0,
            tool_result_cap: 5,
            command_timeout_secs: 99_999,
            tool_models: vec![
                ModelTools {
                    model: " glm-5.3 ".into(),
                    tools: false,
                },
                ModelTools {
                    model: "glm-5.3".into(),
                    tools: true,
                },
                ModelTools {
                    model: " ".into(),
                    tools: true,
                },
            ],
            ..AgentSettings::default()
        }
        .normalized();
        assert_eq!(agent.max_iterations, 1);
        assert_eq!(agent.tool_result_cap, 1_000);
        assert_eq!(agent.command_timeout_secs, 3_600);
        assert_eq!(agent.tools_override("glm-5.3"), Some(false));
        assert_eq!(agent.tools_override("other"), None);
        assert_eq!(agent.tool_models.len(), 1);
    }
}
