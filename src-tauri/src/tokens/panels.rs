//! Which AgentFlow panel produced a CLI's usage: the panel whose agent process (same CLI)
//! ran in the same working directory while the usage happened. Usage that matches no panel
//! still counts, under the workspace that contains its directory.

use std::path::{Component, Path, PathBuf};

use super::cli::CliKind;

/// Grace after a process exits: the CLI may write its last usage line a moment later.
const EXIT_GRACE_MS: u64 = 30_000;
/// Finished sessions kept for late lines and restarts of the reader.
const MAX_ENDED: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelSession {
    pub panel: String,
    pub kind: CliKind,
    pub cwd: PathBuf,
    pub workspace: Option<String>,
    pub started: u64,
    /// `None` while running.
    pub ended: Option<u64>,
}

#[derive(Debug, Default)]
pub struct PanelRegistry {
    sessions: Vec<PanelSession>,
    /// Open workspace folders (for usage that matches no panel).
    workspaces: Vec<String>,
}

impl PanelRegistry {
    pub fn start(&mut self, session: PanelSession) {
        // A panel runs one process at a time: a restart closes the previous window.
        for old in &mut self.sessions {
            if old.panel == session.panel && old.ended.is_none() {
                old.ended = Some(session.started);
            }
        }
        self.sessions.push(session);
        let ended = self.sessions.iter().filter(|s| s.ended.is_some()).count();
        if ended > MAX_ENDED {
            let mut drop = ended - MAX_ENDED;
            self.sessions.retain(|s| {
                let remove = drop > 0 && s.ended.is_some();
                if remove {
                    drop -= 1;
                }
                !remove
            });
        }
    }

    pub fn end(&mut self, panel: &str, at: u64) {
        for session in &mut self.sessions {
            if session.panel == panel && session.ended.is_none() {
                session.ended = Some(at);
            }
        }
    }

    pub fn note_workspace(&mut self, workspace: &str) {
        if !self.workspaces.iter().any(|w| w == workspace) {
            self.workspaces.push(workspace.to_owned());
        }
    }

    /// (panel, workspace) for usage of `kind` in `cwd` at `at`.
    pub fn attribute(
        &self,
        kind: CliKind,
        cwd: Option<&str>,
        at: u64,
    ) -> (Option<String>, Option<String>) {
        let Some(cwd) = cwd.map(normalize) else {
            return (None, None);
        };
        let matched = self
            .sessions
            .iter()
            .filter(|s| s.kind == kind && normalize_path(&s.cwd) == cwd)
            .filter(|s| s.started <= at && s.ended.is_none_or(|end| at <= end + EXIT_GRACE_MS))
            .max_by_key(|s| s.started);
        if let Some(session) = matched {
            let workspace = session
                .workspace
                .clone()
                .or_else(|| self.workspace_of(&cwd));
            return (Some(session.panel.clone()), workspace);
        }
        (None, self.workspace_of(&cwd).or(Some(cwd)))
    }

    /// The deepest known workspace that contains `cwd`.
    fn workspace_of(&self, cwd: &str) -> Option<String> {
        let known = self
            .sessions
            .iter()
            .filter_map(|s| s.workspace.as_deref())
            .chain(self.workspaces.iter().map(String::as_str));
        known
            .filter(|w| is_inside(cwd, &normalize(w)))
            .max_by_key(|w| w.len())
            .map(str::to_owned)
    }
}

/// Paths compare without `.`/`..`, trailing separators or (on Windows) case and slash style.
pub fn normalize(path: &str) -> String {
    normalize_path(Path::new(path))
}

fn normalize_path(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut prefix = String::new();
    for component in path.components() {
        match component {
            Component::Prefix(p) => prefix = p.as_os_str().to_string_lossy().into_owned(),
            Component::RootDir => {}
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
        }
    }
    let joined = format!("{prefix}/{}", parts.join("/"));
    if cfg!(windows) {
        joined.replace('\\', "/").to_lowercase()
    } else {
        joined
    }
}

fn is_inside(path: &str, root: &str) -> bool {
    path == root || path.starts_with(&format!("{}/", root.trim_end_matches('/')))
}

#[cfg(test)]
#[path = "panels_tests.rs"]
mod tests;
