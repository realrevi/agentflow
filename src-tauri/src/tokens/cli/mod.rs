//! Read-only usage readers for the agent CLIs' own session logs (formats verified against
//! each CLI's source; see `parse.rs`):
//! - Claude Code: `$CLAUDE_CONFIG_DIR` or `~/.claude`, `projects/<encoded cwd>/<session>.jsonl`
//!   (subagents under `<session>/subagents/`).
//! - Codex: `$CODEX_HOME` or `~/.codex`, `sessions/YYYY/MM/DD/rollout-*.jsonl`. Cold files
//!   compressed to `.jsonl.zst` (older than 7 days) are not read.
//! - Gemini CLI: `$GEMINI_CLI_HOME` or `~`, `.gemini/tmp/<project>/chats/session-*.jsonl`
//!   (older versions wrote one `.json` per session); `<project>/.project_root` holds the path.
//!
//! Files are tailed from stored byte offsets (`offsets.rs`), so restarts never double count,
//! and every response is deduplicated by its message/response id.

mod ingest;
mod offsets;
mod parse;
mod reader;
mod watch;

use std::path::{Path, PathBuf};

pub use ingest::{gemini_project_root, CliReaders};
pub use parse::LineUsage;
pub use reader::tail_lines;
pub use watch::{start as watch, CliWatch, Pace};
#[cfg(test)]
use {offsets::OffsetStore, reader::read_file};

use super::record::Source;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CliKind {
    Claude,
    Codex,
    Gemini,
}

impl CliKind {
    pub const ALL: [CliKind; 3] = [CliKind::Claude, CliKind::Codex, CliKind::Gemini];

    pub fn source(self) -> Source {
        match self {
            Self::Claude => Source::ClaudeCode,
            Self::Codex => Source::Codex,
            Self::Gemini => Source::GeminiCli,
        }
    }

    /// The model vendor, stored as the record's preset.
    pub fn vendor(self) -> &'static str {
        match self {
            Self::Claude => "anthropic",
            Self::Codex => "openai",
            Self::Gemini => "google",
        }
    }

    /// Whether `path` is one of this CLI's session files.
    pub fn wants(self, path: &Path) -> bool {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        match self {
            Self::Claude => name.ends_with(".jsonl"),
            Self::Codex => name.starts_with("rollout-") && name.ends_with(".jsonl"),
            Self::Gemini => {
                let in_chats = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .is_some_and(|n| n == "chats");
                in_chats
                    && name.starts_with("session-")
                    && (name.ends_with(".jsonl") || name.ends_with(".json"))
            }
        }
    }
}

/// Where each CLI keeps its sessions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliRoots {
    pub claude: Option<PathBuf>,
    pub codex: Option<PathBuf>,
    pub gemini: Option<PathBuf>,
}

impl CliRoots {
    /// The documented locations, honoring each CLI's home-directory override.
    pub fn detect(home: Option<&Path>) -> Self {
        let env = |name: &str| {
            std::env::var_os(name)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        let home = home.map(Path::to_path_buf);
        Self {
            claude: env("CLAUDE_CONFIG_DIR")
                .or_else(|| home.as_ref().map(|h| h.join(".claude")))
                .map(|d| d.join("projects")),
            codex: env("CODEX_HOME")
                .or_else(|| home.as_ref().map(|h| h.join(".codex")))
                .map(|d| d.join("sessions")),
            gemini: env("GEMINI_CLI_HOME")
                .or(home)
                .map(|d| d.join(".gemini").join("tmp")),
        }
    }

    pub fn root(&self, kind: CliKind) -> Option<&Path> {
        match kind {
            CliKind::Claude => self.claude.as_deref(),
            CliKind::Codex => self.codex.as_deref(),
            CliKind::Gemini => self.gemini.as_deref(),
        }
    }

    /// The CLI a changed file belongs to, if it is a session file.
    pub fn kind_of(&self, path: &Path) -> Option<CliKind> {
        CliKind::ALL.into_iter().find(|kind| {
            self.root(*kind)
                .is_some_and(|root| path.starts_with(root) && kind.wants(path))
        })
    }
}

#[cfg(test)]
mod tests;
