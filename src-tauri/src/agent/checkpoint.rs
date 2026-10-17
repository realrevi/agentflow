//! Per-turn checkpoints for "undo this turn": before the first write or edit of a file in
//! a turn, its original bytes (or "did not exist") are saved under
//! `<app data>/checkpoints/<workspace>/<conversation>/<turn>/`. Restoring writes the
//! originals back and removes files the turn created.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::fs::WorkspaceRoot;
use crate::util::{lock, now_ms, workspace_key, write_atomic};

const MANIFEST: &str = "manifest.json";
/// Turns kept per conversation (older checkpoints are deleted).
const KEEP_TURNS: usize = 30;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    turn: String,
    at: u64,
    workspace: String,
    files: Vec<Entry>,
    /// Set when the turn was undone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reverted_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    path: String,
    existed: bool,
    /// File name of the saved original inside the turn folder.
    blob: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointSummary {
    /// The id of the user message that started the turn.
    pub turn: String,
    pub at: u64,
    pub files: Vec<String>,
    /// When the turn was undone (`None` = its changes are still in place).
    pub reverted_at: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReport {
    /// Files put back to their original content.
    pub restored: Vec<String>,
    /// Files the turn created, now removed.
    pub removed: Vec<String>,
}

/// Managed state: where checkpoints live.
#[derive(Debug, Clone)]
pub struct CheckpointStore {
    root: PathBuf,
}

/// One turn's checkpoint, shared by its tool runs.
#[derive(Debug, Clone)]
pub struct TurnCheckpoint {
    dir: PathBuf,
    manifest: Arc<Mutex<Manifest>>,
}

impl CheckpointStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn conversation_dir(&self, workspace: Option<&str>, conversation: &str) -> PathBuf {
        self.root.join(workspace_key(workspace)).join(conversation)
    }

    /// The checkpoint of `turn` (files are saved lazily, on the first write).
    pub fn turn(
        &self,
        workspace: Option<&str>,
        conversation: &str,
        turn: &str,
        root: &Path,
    ) -> TurnCheckpoint {
        let dir = self.conversation_dir(workspace, conversation).join(turn);
        let manifest = read_manifest(&dir).unwrap_or_else(|| Manifest {
            turn: turn.to_owned(),
            at: now_ms(),
            workspace: root.to_string_lossy().into_owned(),
            files: Vec::new(),
            reverted_at: None,
        });
        TurnCheckpoint {
            dir,
            manifest: Arc::new(Mutex::new(manifest)),
        }
    }

    /// Turns with saved files, newest first.
    pub fn list(&self, workspace: Option<&str>, conversation: &str) -> Vec<CheckpointSummary> {
        let mut out: Vec<CheckpointSummary> =
            turn_dirs(&self.conversation_dir(workspace, conversation))
                .iter()
                .filter_map(|dir| read_manifest(dir))
                .filter(|m| !m.files.is_empty())
                .map(|m| CheckpointSummary {
                    turn: m.turn,
                    at: m.at,
                    files: m.files.into_iter().map(|f| f.path).collect(),
                    reverted_at: m.reverted_at,
                })
                .collect();
        out.sort_by_key(|summary| std::cmp::Reverse(summary.at));
        out
    }

    /// Puts the turn's files back. `remove` deletes files the turn created (the OS trash
    /// in the app).
    pub fn restore(
        &self,
        workspace: Option<&str>,
        conversation: &str,
        turn: &str,
        remove: impl Fn(&Path) -> AppResult<()>,
    ) -> AppResult<RestoreReport> {
        let dir = self.conversation_dir(workspace, conversation).join(turn);
        let mut manifest =
            read_manifest(&dir).ok_or_else(|| AppError::NotFound(format!("checkpoint {turn}")))?;
        let root = WorkspaceRoot::open(Path::new(&manifest.workspace))?;
        let mut report = RestoreReport::default();
        for entry in &manifest.files {
            let target = match root.resolve(&entry.path) {
                Err(AppError::NotFound(_)) => root.resolve_new(&entry.path)?,
                other => other?,
            };
            match (&entry.blob, entry.existed) {
                (Some(blob), true) => {
                    write_atomic(&target, &fs::read(dir.join(blob))?)?;
                    report.restored.push(entry.path.clone());
                }
                (_, false) => {
                    if target.exists() {
                        remove(&target)?;
                    }
                    report.removed.push(entry.path.clone());
                }
                (None, true) => {}
            }
        }
        manifest.reverted_at = Some(now_ms());
        write_atomic(&dir.join(MANIFEST), &serde_json::to_vec(&manifest)?)?;
        Ok(report)
    }
}

impl TurnCheckpoint {
    /// Saves `path`'s current content (once per file and turn) before it is changed.
    pub fn save_original(&self, rel: &str, path: &Path) -> AppResult<()> {
        let mut manifest = lock(&self.manifest);
        if manifest.files.iter().any(|f| f.path == rel) {
            return Ok(());
        }
        fs::create_dir_all(&self.dir)?;
        if manifest.files.is_empty() {
            prune(self.dir.parent());
        }
        let existed = path.is_file();
        let blob = if existed {
            let name = format!("{}.orig", manifest.files.len());
            fs::copy(path, self.dir.join(&name))?;
            Some(name)
        } else {
            None
        };
        manifest.files.push(Entry {
            path: rel.to_owned(),
            existed,
            blob,
        });
        write_atomic(&self.dir.join(MANIFEST), &serde_json::to_vec(&*manifest)?)?;
        Ok(())
    }
}

fn read_manifest(dir: &Path) -> Option<Manifest> {
    let bytes = fs::read(dir.join(MANIFEST)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn turn_dirs(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default()
}

/// Keeps the newest `KEEP_TURNS` turns of a conversation.
fn prune(conversation_dir: Option<&Path>) {
    let Some(dir) = conversation_dir else { return };
    let mut turns: Vec<(u64, PathBuf)> = turn_dirs(dir)
        .into_iter()
        .filter_map(|path| read_manifest(&path).map(|m| (m.at, path)))
        .collect();
    if turns.len() < KEEP_TURNS {
        return;
    }
    turns.sort_by_key(|turn| std::cmp::Reverse(turn.0));
    for (_, old) in turns.into_iter().skip(KEEP_TURNS - 1) {
        let _ = fs::remove_dir_all(old);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_dir::TestDir;

    #[test]
    fn restore_puts_files_back_and_removes_created_ones() {
        let dir = TestDir::new("checkpoint");
        let work = dir.path().join("work");
        fs::create_dir_all(work.join("src")).expect("mkdir");
        fs::write(work.join("src/a.txt"), "original").expect("write");
        let store = CheckpointStore::new(dir.path().join("checkpoints"));
        let root = WorkspaceRoot::open(&work).expect("root");
        let turn = store.turn(Some("/w"), "c1", "m1", root.path());

        let a = root.path().join("src/a.txt");
        turn.save_original("src/a.txt", &a).expect("save");
        fs::write(&a, "changed").expect("change");
        turn.save_original("src/a.txt", &a)
            .expect("only the first save counts");
        fs::write(&a, "changed twice").expect("change");
        let created = root.path().join("new.txt");
        turn.save_original("new.txt", &created).expect("save");
        fs::write(&created, "made by the agent").expect("create");

        let listed = store.list(Some("/w"), "c1");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].files, ["src/a.txt", "new.txt"]);
        assert_eq!(listed[0].reverted_at, None);

        let report = store
            .restore(Some("/w"), "c1", "m1", |path| Ok(fs::remove_file(path)?))
            .expect("restore");
        assert_eq!(report.restored, ["src/a.txt"]);
        assert_eq!(report.removed, ["new.txt"]);
        assert_eq!(fs::read_to_string(&a).expect("read"), "original");
        assert!(!created.exists());
        assert!(
            store.list(Some("/w"), "c1")[0].reverted_at.is_some(),
            "the turn shows as undone"
        );
        assert!(store
            .restore(Some("/w"), "c1", "missing", |_| Ok(()))
            .is_err());
    }
}
