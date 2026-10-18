//! The CLI readers as one unit: the session roots, the stored read offsets and the folder
//! watcher. `read` returns the new usage of changed files; the tracker attributes it to
//! panels and writes it to the ledger, then `save`s the offsets.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::offsets::OffsetStore;
use super::parse::LineUsage;
use super::reader::read_file;
use super::watch::CliWatch;
use super::{CliKind, CliRoots};

/// Folder depth searched on the startup scan (Codex: `sessions/YYYY/MM/DD/file`).
const MAX_DEPTH: usize = 6;

pub struct CliReaders {
    pub roots: CliRoots,
    offsets: OffsetStore,
    pub watch: Option<CliWatch>,
}

impl CliReaders {
    pub fn new(roots: CliRoots, offsets: PathBuf) -> Self {
        Self {
            roots,
            offsets: OffsetStore::load(offsets),
            watch: None,
        }
    }

    /// New usage in `paths` (files of other folders are ignored).
    pub fn read(&mut self, paths: &[PathBuf]) -> Vec<(CliKind, LineUsage)> {
        let mut out = Vec::new();
        for path in paths {
            let Some(kind) = self.roots.kind_of(path) else {
                continue;
            };
            let result = match read_file(kind, path, self.offsets.get(path)) {
                Ok(result) => result,
                Err(err) => {
                    tracing::debug!("cannot read {}: {err}", path.display());
                    continue;
                }
            };
            self.offsets.set(path, result.state);
            let project = (kind == CliKind::Gemini)
                .then(|| gemini_project_root(path))
                .flatten();
            for mut usage in result.usage {
                if usage.cwd.is_none() {
                    usage.cwd.clone_from(&project);
                }
                out.push((kind, usage));
            }
        }
        out
    }

    pub fn save(&mut self) {
        self.offsets.save();
    }

    /// Session files changed since `since_ms` (the startup scan).
    pub fn recent_files(&self, since_ms: u64) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for kind in CliKind::ALL {
            if let Some(root) = self.roots.root(kind) {
                files_under(root, kind, since_ms, &mut out);
            }
        }
        out
    }
}

/// `kind`'s session files under `root` changed since `since_ms` (0 = all).
pub(super) fn files_under(root: &Path, kind: CliKind, since_ms: u64, out: &mut Vec<PathBuf>) {
    collect(root, kind, since_ms, 0, out);
}

fn collect(dir: &Path, kind: CliKind, since_ms: u64, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            if depth < MAX_DEPTH {
                collect(&path, kind, since_ms, depth + 1, out);
            }
            continue;
        }
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis() as u64);
        if modified >= since_ms && kind.wants(&path) {
            out.push(path);
        }
    }
}

/// Gemini CLI writes the project folder into `<project>/.project_root` (sessions are in
/// `<project>/chats/`); its lines carry no cwd.
pub fn gemini_project_root(session: &Path) -> Option<String> {
    let project = session.parent()?.parent()?;
    let text = std::fs::read_to_string(project.join(".project_root")).ok()?;
    let root = text.trim();
    (!root.is_empty()).then(|| root.to_owned())
}
