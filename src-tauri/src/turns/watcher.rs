//! Managed state: the agent CLI sessions running in panels, the log files matched to them
//! and each panel's turn fold. The phase 6 folder watcher runs (at the live pace) only while
//! a CLI panel runs. A file belongs to the panel whose process (same CLI) runs in the same
//! folder; lines older than that process are history and say nothing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};

use super::events::{TurnEvent, TurnSink};
use super::fold::{Fold, Out};
use super::parse;
use crate::tokens::cli::{self, gemini_project_root, tail_lines, CliKind, CliRoots, CliWatch};
use crate::tokens::panels::normalize;
use crate::util::{lock, now_ms};

/// Lines may be stamped a moment before the process was registered.
const START_SLACK_MS: u64 = 2_000;
/// The CLI may write its last lines a moment after it exits.
const EXIT_GRACE_MS: u64 = 30_000;
/// A file seen for the first time is read from its last part only when it is large.
const FIRST_READ_MAX: u64 = 8 * 1024 * 1024;
const FIRST_READ_TAIL: u64 = 1024 * 1024;

struct Session {
    panel: String,
    kind: CliKind,
    /// Normalized.
    cwd: String,
    started: u64,
    ended: Option<u64>,
    file: Option<PathBuf>,
    fold: Fold,
}

impl Session {
    fn covers(&self, kind: CliKind, cwd: &str, at: u64) -> bool {
        self.kind == kind
            && self.cwd == cwd
            && self.started <= at + START_SLACK_MS
            && self.ended.is_none_or(|end| at <= end + EXIT_GRACE_MS)
    }
}

struct Tail {
    kind: CliKind,
    offset: u64,
    cwd: Option<String>,
}

#[derive(Default)]
struct State {
    sessions: Vec<Session>,
    files: HashMap<PathBuf, Tail>,
    watch: Option<CliWatch>,
    sink: Option<TurnSink>,
}

#[derive(Clone)]
pub struct TurnWatcher {
    inner: Arc<Inner>,
}

struct Inner {
    roots: CliRoots,
    /// False in tests: `ingest` is called directly.
    watch_files: bool,
    state: Mutex<State>,
}

/// Whether the CLI's log marks the end of a turn.
fn exact(kind: CliKind) -> bool {
    kind != CliKind::Gemini
}

impl TurnWatcher {
    pub fn new(roots: CliRoots) -> Self {
        Self::build(roots, true)
    }

    #[cfg(test)]
    pub fn without_watch(roots: CliRoots) -> Self {
        Self::build(roots, false)
    }

    fn build(roots: CliRoots, watch_files: bool) -> Self {
        Self {
            inner: Arc::new(Inner {
                roots,
                watch_files,
                state: Mutex::new(State::default()),
            }),
        }
    }

    /// The frontend's Channel (replaces the previous one).
    pub fn subscribe(&self, sink: TurnSink) {
        lock(&self.inner.state).sink = Some(sink);
    }

    /// An agent CLI starts in `panel` (call before spawning it).
    pub fn panel_started(&self, panel: &str, kind: CliKind, cwd: &Path) {
        let mut state = lock(&self.inner.state);
        let now = now_ms();
        for old in &mut state.sessions {
            if old.panel == panel && old.ended.is_none() {
                old.ended = Some(now);
            }
        }
        state
            .sessions
            .retain(|s| s.ended.is_none_or(|end| now <= end + EXIT_GRACE_MS));
        state.sessions.push(Session {
            panel: panel.to_owned(),
            kind,
            cwd: normalize(&cwd.to_string_lossy()),
            started: now,
            ended: None,
            file: None,
            fold: Fold::new(exact(kind)),
        });
        if self.inner.watch_files && state.watch.is_none() {
            let weak: Weak<Inner> = Arc::downgrade(&self.inner);
            state.watch = cli::watch(&self.inner.roots, cli::Pace::LIVE, move |paths| {
                if let Some(inner) = weak.upgrade() {
                    TurnWatcher { inner }.ingest(&paths);
                }
            });
        }
    }

    /// The process in `panel` exited. The watcher stops once no CLI panel runs.
    pub fn panel_ended(&self, panel: &str) {
        let mut state = lock(&self.inner.state);
        let now = now_ms();
        for session in &mut state.sessions {
            if session.panel == panel && session.ended.is_none() {
                session.ended = Some(now);
            }
        }
        if state.sessions.iter().all(|s| s.ended.is_some()) {
            state.watch = None;
            state.files.clear();
        }
    }

    pub fn shutdown(&self) {
        lock(&self.inner.state).watch = None;
    }

    /// Reads what is new in changed log files and sends the events.
    pub fn ingest(&self, paths: &[PathBuf]) {
        let mut state = lock(&self.inner.state);
        let mut events = Vec::new();
        for path in paths {
            self.read(&mut state, path, &mut events);
        }
        if let Some(sink) = &state.sink {
            // Channel sends never block, so the sink is called under the lock.
            for event in events {
                sink(event);
            }
        }
    }

    fn read(&self, state: &mut State, path: &Path, events: &mut Vec<TurnEvent>) {
        let Some(kind) = self.inner.roots.kind_of(path) else {
            return;
        };
        let subagent = path.components().any(|c| c.as_os_str() == "subagents");
        let legacy = path.extension().is_some_and(|ext| ext == "json");
        if subagent || legacy || state.sessions.iter().all(|s| s.kind != kind) {
            return;
        }
        let tail = state.files.entry(path.to_path_buf()).or_insert_with(|| {
            let len = std::fs::metadata(path).map_or(0, |m| m.len());
            let offset = if len > FIRST_READ_MAX {
                len - FIRST_READ_TAIL
            } else {
                0
            };
            let cwd = (kind == CliKind::Gemini)
                .then(|| gemini_project_root(path))
                .flatten();
            Tail { kind, offset, cwd }
        });
        let mut lines = Vec::new();
        let read = tail_lines(path, tail.offset, |text| {
            match parse::line(tail.kind, text) {
                Ok(line) => lines.push(line),
                Err(err) => tracing::debug!("turns: skipping a line in {}: {err}", path.display()),
            }
        });
        match read {
            Ok(offset) => tail.offset = offset,
            Err(err) => {
                tracing::debug!("turns: cannot read {}: {err}", path.display());
                return;
            }
        }
        let mut touched: Vec<usize> = Vec::new();
        for line in lines {
            if line.cwd.is_some() {
                tail.cwd.clone_from(&line.cwd);
            }
            if line.signals.is_empty() {
                continue;
            }
            let Some(cwd) = tail.cwd.as_deref() else {
                continue;
            };
            let at = if line.at == 0 { now_ms() } else { line.at };
            let Some(index) = owner(&state.sessions, kind, &normalize(cwd), at, path) else {
                continue;
            };
            let session = &mut state.sessions[index];
            if session.file.as_deref() != Some(path) {
                session.file = Some(path.to_path_buf());
                events.push(TurnEvent::Attached {
                    panel: session.panel.clone(),
                    exact: session.fold.exact,
                });
            }
            let mut out = Vec::new();
            for signal in line.signals {
                session.fold.apply(signal, Some(cwd), &mut out);
            }
            events.extend(out.into_iter().map(|o| event(&session.panel, o)));
            if !touched.contains(&index) {
                touched.push(index);
            }
        }
        for index in touched {
            let session = &mut state.sessions[index];
            let mut out = Vec::new();
            session.fold.settle(&mut out);
            events.extend(out.into_iter().map(|o| event(&session.panel, o)));
        }
    }
}

/// The session a line belongs to: same CLI and folder, running at `at`; one already reading
/// this file wins, then one without a file, then the newest.
fn owner(sessions: &[Session], kind: CliKind, cwd: &str, at: u64, path: &Path) -> Option<usize> {
    sessions
        .iter()
        .enumerate()
        .filter(|(_, s)| s.covers(kind, cwd, at))
        .max_by_key(|(_, s)| {
            let rank = match s.file.as_deref() {
                Some(file) if file == path => 2,
                None => 1,
                Some(_) => 0,
            };
            (rank, s.started)
        })
        .map(|(index, _)| index)
}

fn event(panel: &str, out: Out) -> TurnEvent {
    let panel = panel.to_owned();
    match out {
        Out::Activity {
            kind,
            action,
            target,
        } => TurnEvent::Activity {
            panel,
            kind,
            action,
            target,
        },
        Out::Draft(text) => TurnEvent::Draft { panel, text },
        Out::Ended {
            report,
            interrupted,
        } => TurnEvent::TurnEnded {
            panel,
            report,
            interrupted,
        },
    }
}
