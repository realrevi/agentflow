//! Data shared with the frontend (`src/shared/types.ts`). Paths are relative to the
//! workspace root and always use `/` as the separator.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    /// `None` for binary files.
    pub content: Option<String>,
    /// A NUL byte was found in the first 8 KB.
    pub binary: bool,
    /// The file was not valid UTF-8; invalid bytes were replaced with U+FFFD.
    pub lossy: bool,
    pub size: u64,
    /// Modification time (ms since the Unix epoch); pass it back to `fs_write`.
    pub mtime: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStat {
    pub size: u64,
    pub mtime: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryKind {
    File,
    Dir,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickOpenMatch {
    pub path: String,
    /// Indices (in chars) of the matched characters, for highlighting.
    pub positions: Vec<u32>,
}

/// One debounced batch of watcher events.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsChanges {
    /// Every changed path (content or structure), deduplicated.
    pub paths: Vec<String>,
    /// Paths that were created, removed or renamed: their parent folder needs a re-list.
    pub structural: Vec<String>,
    /// Ignore rules changed or events were lost: re-list everything.
    pub rescan: bool,
}
