//! The library's data: sources (from `registry/sources.json` or the user's settings), the
//! catalog of imported items, what could not be imported and why, and each source's status.
//! Mirrored by `src/shared/libraryTypes.ts`.

use serde::{Deserialize, Serialize};

use super::safety::Flag;
use crate::agents::{Target, TemplateKind};

/// The file formats the importers read (docs/library-sources.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Format {
    ClaudeSubagent,
    ClaudeSkill,
    CursorRule,
    CodexAgent,
    GeminiCommand,
    GeminiAgent,
    AgentsMd,
}

impl Format {
    pub fn id(self) -> &'static str {
        match self {
            Self::ClaudeSubagent => "claudeSubagent",
            Self::ClaudeSkill => "claudeSkill",
            Self::CursorRule => "cursorRule",
            Self::CodexAgent => "codexAgent",
            Self::GeminiCommand => "geminiCommand",
            Self::GeminiAgent => "geminiAgent",
            Self::AgentsMd => "agentsMd",
        }
    }
}

/// Which files of a source are items, in which format (`None` = guessed per file).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Format>,
    pub include: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
    /// The tools the wizard preselects (default: the format's own tool).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<Target>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub name: String,
    /// `owner/name` on GitHub.
    pub repo: String,
    /// `None` = the repository's default branch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// SPDX id, or `per-item` (each item carries its own LICENSE file).
    #[serde(default)]
    pub license: String,
    pub entries: Vec<Entry>,
    /// Curated in the AgentFlow registry (custom sources are not).
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFile {
    /// Relative to the item's folder.
    pub path: String,
    pub size: u64,
}

/// One importable agent, skill, rule or command (a card in the Browse tab).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// `<source id>:<path>`.
    pub id: String,
    pub source_id: String,
    pub source_name: String,
    pub repo: String,
    pub verified: bool,
    /// The main file in the repository.
    pub path: String,
    /// Where it is on GitHub (at the imported commit).
    pub url: String,
    pub format: Format,
    pub kind: TemplateKind,
    /// The installed template's id.
    pub template_id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub tags: Vec<String>,
    pub category: Option<String>,
    /// Every tool it can be installed for, and the ones preselected.
    pub targets: Vec<Target>,
    pub native: Vec<Target>,
    /// ≈ tokens of the instructions (phase 6 estimator).
    pub tokens: u64,
    pub flags: Vec<Flag>,
    /// A skill's other files (from the repository tree).
    pub files: Vec<ItemFile>,
    /// Content hash of the item's files: a change upstream means an update.
    pub sha: String,
    pub commit: String,
    /// The source's last commit date (ISO 8601).
    pub updated_at: Option<String>,
    pub stars: u64,
}

/// A file that looked like an item but could not be converted (listed, never dropped).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unsupported {
    pub source_id: String,
    pub path: String,
    /// A stable id the UI translates (`library.unsupported.<reason>`).
    pub reason: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceError {
    /// `rateLimited`, `notFound`, `network`, `invalid` or `http`.
    pub kind: String,
    pub message: String,
    /// When GitHub's rate limit resets (ms since the epoch).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    pub id: String,
    pub name: String,
    pub repo: String,
    pub verified: bool,
    pub license: String,
    pub stars: u64,
    pub commit: Option<String>,
    pub updated_at: Option<String>,
    /// The last successful check (ms since the epoch).
    pub checked_at: Option<u64>,
    pub imported: usize,
    pub unsupported: usize,
    pub error: Option<SourceError>,
}

/// GitHub's REST rate limit as last seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rate {
    pub limit: u64,
    pub remaining: u64,
    /// ms since the epoch.
    pub reset_at: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    /// The last refresh that reached GitHub (ms since the epoch).
    pub refreshed_at: Option<u64>,
    pub sources: Vec<SourceStatus>,
    pub items: Vec<Item>,
    pub unsupported: Vec<Unsupported>,
    pub rate: Option<Rate>,
    /// The registry came from GitHub (`remote`), the cache, or the app (`bundled`).
    #[serde(default)]
    pub registry: String,
}
