//! Managed state: loads a workspace's history on first use, caps it, writes atomically.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use super::model::{PromptEntry, WorkspaceHistory, FORMAT_VERSION};
use crate::error::{AppError, AppResult};
use crate::util::{lock, write_atomic};

/// Per panel, oldest entries are dropped beyond this.
pub const MAX_ENTRIES: usize = 200;
/// Per workspace, the panels used least recently are dropped beyond this.
pub const MAX_PANELS: usize = 64;
const MAX_PANEL_KEY: usize = 64;
/// Longer prompts (pasted logs) are stored cut to this many bytes, so the file stays small.
pub const MAX_TEXT_BYTES: usize = 8 * 1024;

#[derive(Clone)]
pub struct PromptHistory {
    dir: PathBuf,
    cache: Arc<Mutex<HashMap<PathBuf, WorkspaceHistory>>>,
}

impl PromptHistory {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Oldest first.
    pub fn list(&self, workspace: Option<&str>, panel: &str) -> AppResult<Vec<PromptEntry>> {
        check_panel(panel)?;
        let path = self.file(workspace);
        let mut cache = lock(&self.cache);
        let history = load_cached(&mut cache, &path);
        Ok(history.panels.get(panel).cloned().unwrap_or_default())
    }

    /// Appends a prompt (repeating the last one only refreshes its time) and saves.
    pub fn add(&self, workspace: Option<&str>, panel: &str, text: &str) -> AppResult<PromptEntry> {
        check_panel(panel)?;
        if text.trim().is_empty() {
            return Err(AppError::InvalidInput("empty prompt".into()));
        }
        let path = self.file(workspace);
        let mut cache = lock(&self.cache);
        let history = load_cached(&mut cache, &path);
        history.version = FORMAT_VERSION;
        history.workspace = workspace.map(str::to_owned);
        let entry = PromptEntry {
            text: truncate(text, MAX_TEXT_BYTES).to_owned(),
            at: now_ms(),
        };
        let entries = history.panels.entry(panel.to_owned()).or_default();
        if entries.last().is_some_and(|last| last.text == entry.text) {
            entries.pop();
        }
        entries.push(entry.clone());
        if entries.len() > MAX_ENTRIES {
            let excess = entries.len() - MAX_ENTRIES;
            entries.drain(..excess);
        }
        prune_panels(history);
        write_atomic(&path, &serde_json::to_vec_pretty(history)?)?;
        Ok(entry)
    }

    fn file(&self, workspace: Option<&str>) -> PathBuf {
        self.dir.join(file_name(workspace))
    }
}

fn load_cached<'a>(
    cache: &'a mut HashMap<PathBuf, WorkspaceHistory>,
    path: &Path,
) -> &'a mut WorkspaceHistory {
    cache
        .entry(path.to_path_buf())
        .or_insert_with(|| read(path))
}

/// A missing file is an empty history; a corrupt one is logged and replaced on next save.
fn read(path: &Path) -> WorkspaceHistory {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|err| {
            tracing::warn!(
                "ignoring unreadable prompt history {}: {err}",
                path.display()
            );
            WorkspaceHistory::default()
        }),
        Err(_) => WorkspaceHistory::default(),
    }
}

/// Keeps the `MAX_PANELS` panels that sent a prompt most recently.
fn prune_panels(history: &mut WorkspaceHistory) {
    while history.panels.len() > MAX_PANELS {
        let oldest = history
            .panels
            .iter()
            .min_by_key(|(_, entries)| entries.last().map_or(0, |entry| entry.at))
            .map(|(key, _)| key.clone());
        match oldest {
            Some(key) => history.panels.remove(&key),
            None => break,
        };
    }
}

/// `<folder>-<hash>.json` for a workspace (readable and collision-safe), `_home.json` without.
pub fn file_name(workspace: Option<&str>) -> String {
    let Some(path) = workspace else {
        return "_home.json".into();
    };
    let folder: String = path
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .take(40)
        .collect();
    format!("{folder}-{:016x}.json", fnv1a(path.as_bytes()))
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The longest prefix of `text` within `max` bytes, cut on a char boundary.
fn truncate(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let end = (0..=max)
        .rev()
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or(0);
    &text[..end]
}

fn check_panel(panel: &str) -> AppResult<()> {
    let valid = !panel.is_empty()
        && panel.len() <= MAX_PANEL_KEY
        && panel
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'));
    if valid {
        Ok(())
    } else {
        Err(AppError::InvalidInput(format!("bad panel key: {panel}")))
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_dir::TestDir;

    const WS: Option<&str> = Some("/home/me/projects/demo-app");

    #[test]
    fn keeps_the_last_200_entries_per_panel() {
        let dir = TestDir::new("prompts");
        let history = PromptHistory::new(dir.path().to_path_buf());
        for i in 0..230 {
            history.add(WS, "p1", &format!("prompt {i}")).expect("add");
        }
        let entries = history.list(WS, "p1").expect("list");
        assert_eq!(entries.len(), MAX_ENTRIES);
        assert_eq!(entries[0].text, "prompt 30");
        assert_eq!(entries[199].text, "prompt 229");
    }

    #[test]
    fn panels_and_workspaces_are_separate() {
        let dir = TestDir::new("prompts");
        let history = PromptHistory::new(dir.path().to_path_buf());
        history.add(WS, "p1", "fix the tests").expect("add");
        history.add(WS, "p2", "write docs").expect("add");
        history.add(None, "p1", "home prompt").expect("add");
        assert_eq!(
            history.list(WS, "p1").expect("list")[0].text,
            "fix the tests"
        );
        assert_eq!(history.list(WS, "p2").expect("list")[0].text, "write docs");
        assert_eq!(
            history.list(None, "p1").expect("list")[0].text,
            "home prompt"
        );
        assert!(history.list(WS, "p3").expect("list").is_empty());
    }

    #[test]
    fn survives_a_restart() {
        let dir = TestDir::new("prompts");
        PromptHistory::new(dir.path().to_path_buf())
            .add(WS, "p1", "line one\nline two")
            .expect("add");
        let reopened = PromptHistory::new(dir.path().to_path_buf());
        let entries = reopened.list(WS, "p1").expect("list");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "line one\nline two");
        assert!(entries[0].at > 0);
        assert!(dir.path().join(file_name(WS)).is_file());
    }

    #[test]
    fn repeats_and_bad_input() {
        let dir = TestDir::new("prompts");
        let history = PromptHistory::new(dir.path().to_path_buf());
        history.add(WS, "p1", "same").expect("add");
        history.add(WS, "p1", "same").expect("add");
        assert_eq!(history.list(WS, "p1").expect("list").len(), 1);
        assert!(history.add(WS, "p1", "   ").is_err());
        assert!(history.add(WS, "../etc", "x").is_err());
        assert!(history.list(WS, "").is_err());
    }

    #[test]
    fn long_prompts_are_cut_on_a_char_boundary() {
        let dir = TestDir::new("prompts");
        let history = PromptHistory::new(dir.path().to_path_buf());
        let long = "ü".repeat(MAX_TEXT_BYTES); // 2 bytes each
        let stored = history.add(WS, "p1", &long).expect("add");
        assert_eq!(stored.text.len(), MAX_TEXT_BYTES);
        assert_eq!(truncate("aü", 2), "a");
        assert_eq!(truncate("short", MAX_TEXT_BYTES), "short");
    }

    #[test]
    fn drops_the_least_recent_panels() {
        let mut ws = WorkspaceHistory::default();
        for i in 0..(MAX_PANELS + 3) {
            let entry = PromptEntry {
                text: "x".into(),
                at: i as u64,
            };
            ws.panels.insert(format!("p{i}"), vec![entry]);
        }
        prune_panels(&mut ws);
        assert_eq!(ws.panels.len(), MAX_PANELS);
        assert!(!ws.panels.contains_key("p0") && !ws.panels.contains_key("p2"));
        assert!(ws.panels.contains_key("p3"));
    }

    #[test]
    fn file_names_are_stable_and_distinct() {
        assert_eq!(file_name(WS), file_name(WS));
        assert!(file_name(WS).starts_with("demo-app-"));
        assert_ne!(file_name(WS), file_name(Some("/other/demo-app")));
        assert_eq!(file_name(None), "_home.json");
        assert!(file_name(Some("C:\\Users\\me\\x y")).starts_with("xy-"));
    }

    #[test]
    fn corrupt_files_start_empty() {
        let dir = TestDir::new("prompts");
        fs::write(dir.path().join(file_name(WS)), "{oops").expect("write");
        let history = PromptHistory::new(dir.path().to_path_buf());
        assert!(history.list(WS, "p1").expect("list").is_empty());
        history.add(WS, "p1", "fresh").expect("add");
        assert_eq!(history.list(WS, "p1").expect("list").len(), 1);
    }
}
