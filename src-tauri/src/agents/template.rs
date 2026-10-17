//! The agent template format (schema 1): one source describes an agent once; renderers
//! turn it plus the user's answers into files for each tool. Documented in
//! docs/agent-template.md; `schemas/agent-template.schema.json` is the JSON Schema of the same
//! rules (a test keeps the two in step). Community templates (phase 9) use this format too.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::conversations::PermissionMode;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Template {
    pub schema: u32,
    /// Lowercase letters, digits and hyphens: also the file name in every tool.
    pub id: String,
    pub name: String,
    /// One line: tools show it to decide when to use the agent.
    pub description: String,
    /// `major.minor.patch`; a higher version offers an update.
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Suggested pixel character id for AgentFlow panels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character: Option<String>,
    /// The language the instructions are written in (BCP 47, e.g. `en`).
    #[serde(default = "english")]
    pub language: String,
    /// The role and rules, Markdown with `{{placeholders}}`.
    pub instructions: String,
    #[serde(default)]
    pub questions: Vec<Question>,
    #[serde(default)]
    pub targets: TargetOptions,
}

fn english() -> String {
    "en".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionKind {
    Text,
    Multiline,
    Choice,
    MultiChoice,
    Color,
    Font,
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Question {
    /// Used as `{{id}}` in the instructions.
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub help: String,
    #[serde(rename = "type")]
    pub kind: QuestionKind,
    /// A string (text, multiline, choice, color, font), a list (multiChoice) or a bool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    /// The choices (choice, multiChoice).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    /// A workspace detector that can prefill the answer (see detect.rs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detect: Option<String>,
    /// An answer is needed (else an empty answer drops the lines that use it).
    #[serde(default)]
    pub required: bool,
    /// Questions with the same group are shown together.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

/// Per-tool options; every target can be installed with the defaults.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TargetOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_code: Option<ClaudeOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex: Option<CodexOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemini: Option<GeminiOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<CursorOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agentflow: Option<AgentflowOptions>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeOptions {
    /// Subagent tools (`Read`, `Grep`, `Bash`...); none = all tools of the session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// `sonnet`, `opus`, `haiku`, `inherit` or a model id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexOptions {}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeminiOptions {}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CursorOptions {
    /// Files the rule attaches to; none = the agent decides from the description.
    #[serde(default)]
    pub globs: Vec<String>,
    #[serde(default)]
    pub always_apply: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentflowOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<PermissionMode>,
    /// AgentFlow tool names; they only narrow what the permission mode allows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// Provider hint (`anthropic`, `openai`, a preset id...) and model hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Answers by question id: a string, a list of strings or a bool.
pub type Answers = std::collections::BTreeMap<String, Value>;
