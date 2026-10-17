//! How far each session file was read, plus the context a resumed read needs (Codex keeps
//! the cwd and model in earlier lines). Saved atomically next to the ledger, so a restart
//! continues where it stopped instead of counting the same lines again.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::util::write_atomic;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FileState {
    /// Bytes consumed (always at a line boundary).
    pub offset: u64,
    /// Modification time (ms) at the last read; whole-file formats re-read on change.
    pub mtime: u64,
    pub cwd: Option<String>,
    pub model: Option<String>,
    /// Codex: the file has `token_usage_record` lines (then `token_count` is ignored).
    pub has_records: bool,
}

#[derive(Debug, Default)]
pub struct OffsetStore {
    path: Option<PathBuf>,
    files: HashMap<String, FileState>,
    dirty: bool,
}

impl OffsetStore {
    /// Loads `path` (a missing or corrupt file starts empty; ids still prevent double counts).
    pub fn load(path: PathBuf) -> Self {
        let files = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            path: Some(path),
            files,
            dirty: false,
        }
    }

    pub fn get(&self, file: &Path) -> FileState {
        self.files.get(&key(file)).cloned().unwrap_or_default()
    }

    pub fn set(&mut self, file: &Path, state: FileState) {
        let key = key(file);
        if self.files.get(&key) != Some(&state) {
            self.files.insert(key, state);
            self.dirty = true;
        }
    }

    /// Writes pending changes. Entries for files that no longer exist are dropped.
    pub fn save(&mut self) {
        if !self.dirty {
            return;
        }
        self.files.retain(|path, _| Path::new(path).exists());
        let Some(path) = &self.path else { return };
        match serde_json::to_vec(&self.files) {
            Ok(bytes) => match write_atomic(path, &bytes) {
                Ok(()) => self.dirty = false,
                Err(err) => tracing::warn!("cannot save CLI read offsets: {err}"),
            },
            Err(err) => tracing::warn!("cannot encode CLI read offsets: {err}"),
        }
    }
}

fn key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
