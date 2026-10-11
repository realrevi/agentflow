//! Types shared by the PTY module and the command layer (mirrored in `src/shared/types.ts`).

use serde::{Deserialize, Serialize};

pub type SessionId = u64;

/// What a terminal panel runs. Agent CLIs are resolved on PATH at spawn time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PtyKind {
    Shell,
    Claude,
    Codex,
    Gemini,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermSize {
    pub cols: u16,
    pub rows: u16,
}

impl TermSize {
    const MIN: u16 = 2;
    const MAX: u16 = 1000;

    /// Clamps to a sane range; a zero-sized pty confuses most programs.
    pub fn clamped(cols: u16, rows: u16) -> Self {
        Self {
            cols: cols.clamp(Self::MIN, Self::MAX),
            rows: rows.clamp(Self::MIN, Self::MAX),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: SessionId,
    pub kind: PtyKind,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub pid: Option<u32>,
}

/// How the child ended. `code` is `None` if the exit status could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitInfo {
    pub code: Option<u32>,
    pub signal: Option<String>,
}

/// What a session emits: coalesced raw output bytes, then exactly one `Exit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyOutput {
    Data(Vec<u8>),
    Exit(ExitInfo),
}

/// Receives a session's output on the pump thread. Dropped when the session ends.
pub type OutputSink = Box<dyn FnMut(PtyOutput) + Send>;
