//! Watches the CLI session folders with `notify` (no polling) and hands over the changed
//! session files in batches, once writes pause for `Pace::quiet` (at most every
//! `Pace::max_wait`).
//! A folder that does not exist yet (the CLI was never used) is waited for by watching its
//! nearest existing ancestor non-recursively; once it appears it is watched recursively and
//! the files already in it are handed over too.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::ingest::files_under;
use super::{CliKind, CliRoots};

/// How changes are batched: usage can wait; live activity (turn detection) cannot.
#[derive(Debug, Clone, Copy)]
pub struct Pace {
    pub quiet: Duration,
    pub max_wait: Duration,
}

impl Pace {
    pub const USAGE: Pace = Pace {
        quiet: Duration::from_millis(500),
        max_wait: Duration::from_secs(3),
    };
    pub const LIVE: Pace = Pace {
        quiet: Duration::from_millis(120),
        max_wait: Duration::from_millis(800),
    };
}

enum Input {
    Event(notify::Result<Event>),
    Stop,
}

/// Stops the watcher thread when dropped.
pub struct CliWatch {
    input: mpsc::Sender<Input>,
}

impl Drop for CliWatch {
    fn drop(&mut self) {
        let _ = self.input.send(Input::Stop);
    }
}

/// What the watcher thread watches: session roots (recursive) and, for roots that do not
/// exist yet, the ancestor they will appear in (non-recursive).
struct Watching {
    watcher: RecommendedWatcher,
    /// Roots not created yet, with the ancestor currently watched for them.
    waiting: Vec<(CliKind, PathBuf, PathBuf)>,
}

/// How a root is watched after `Watching::add`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Added {
    /// The root itself, recursively.
    Root,
    /// Its closest existing ancestor, until the root appears.
    Ancestor,
    Failed,
}

impl Watching {
    /// Watches `root`, or its closest existing ancestor. A folder that appears while this
    /// runs (between a check and the watch) is caught by checking again after watching.
    fn add(&mut self, kind: CliKind, root: &Path) -> Added {
        loop {
            if root.is_dir() {
                return match self.watcher.watch(root, RecursiveMode::Recursive) {
                    Ok(()) => Added::Root,
                    Err(err) => {
                        tracing::warn!("not watching {}: {err}", root.display());
                        Added::Failed
                    }
                };
            }
            let Some(ancestor) = closest_existing(root) else {
                return Added::Failed;
            };
            let already = self.waiting.iter().any(|(_, _, a)| *a == ancestor);
            if !already {
                if let Err(err) = self.watcher.watch(&ancestor, RecursiveMode::NonRecursive) {
                    tracing::debug!("not watching {}: {err}", ancestor.display());
                    return Added::Failed;
                }
            }
            let settled = !root.is_dir() && closest_existing(root).as_ref() == Some(&ancestor);
            if settled {
                self.waiting.push((kind, root.to_path_buf(), ancestor));
                return Added::Ancestor;
            }
            // A deeper folder appeared meanwhile: its creation event may be lost, so look
            // again instead of waiting for it.
            if !already {
                let _ = self.watcher.unwatch(&ancestor);
            }
        }
    }

    /// Something was created under a watched ancestor: roots that now exist (or got a
    /// closer existing ancestor) move on. Returns the files of roots that appeared.
    fn created(&mut self, path: &Path) -> Vec<PathBuf> {
        let (affected, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.waiting)
            .into_iter()
            .partition(|(_, root, _)| root.starts_with(path));
        self.waiting = keep;
        let mut files = Vec::new();
        for (kind, root, ancestor) in affected {
            if !self.waiting.iter().any(|(_, _, a)| *a == ancestor) {
                let _ = self.watcher.unwatch(&ancestor);
            }
            // Files written before the recursive watch started are only found by a scan.
            if self.add(kind, &root) == Added::Root {
                files_under(&root, kind, 0, &mut files);
            }
        }
        files
    }
}

fn closest_existing(root: &Path) -> Option<PathBuf> {
    root.ancestors()
        .skip(1)
        .find(|dir| dir.is_dir())
        .map(Path::to_path_buf)
}

/// Starts watching every root (or where it will appear). `on_batch` runs on the watcher
/// thread.
pub fn start(
    roots: &CliRoots,
    pace: Pace,
    on_batch: impl FnMut(Vec<PathBuf>) + Send + 'static,
) -> Option<CliWatch> {
    let (input, events) = mpsc::channel();
    let forward = input.clone();
    let watcher = match notify::recommended_watcher(move |event| {
        let _ = forward.send(Input::Event(event));
    }) {
        Ok(watcher) => watcher,
        Err(err) => {
            tracing::warn!("cannot watch CLI sessions: {err}");
            return None;
        }
    };
    let mut watching = Watching {
        watcher,
        waiting: Vec::new(),
    };
    let mut any = false;
    for kind in CliKind::ALL {
        if let Some(root) = roots.root(kind) {
            any |= watching.add(kind, root) != Added::Failed;
        }
    }
    if !any {
        return None;
    }
    let roots = roots.clone();
    let spawned = std::thread::Builder::new()
        .name("agentflow-cli-usage".into())
        .spawn(move || run(watching, &events, &roots, pace, on_batch));
    match spawned {
        Ok(_) => Some(CliWatch { input }),
        Err(err) => {
            tracing::warn!("cannot start the CLI usage watcher: {err}");
            None
        }
    }
}

fn run(
    mut watching: Watching,
    events: &mpsc::Receiver<Input>,
    roots: &CliRoots,
    pace: Pace,
    mut on_batch: impl FnMut(Vec<PathBuf>),
) {
    let mut pending: BTreeSet<PathBuf> = BTreeSet::new();
    let mut first: Option<Instant> = None;
    let mut last = Instant::now();
    loop {
        let input = match first {
            Some(first) => {
                let deadline = (last + pace.quiet).min(first + pace.max_wait);
                match events.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                    Ok(input) => Some(input),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            None => match events.recv() {
                Ok(input) => Some(input),
                Err(_) => return,
            },
        };
        match input {
            Some(Input::Stop) => return,
            Some(Input::Event(Ok(event))) => {
                let mut changed: Vec<PathBuf> = Vec::new();
                if matches!(event.kind, EventKind::Create(_)) && !watching.waiting.is_empty() {
                    for path in &event.paths {
                        changed.extend(watching.created(path));
                    }
                }
                if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                    changed.extend(event.paths);
                }
                for path in changed {
                    if roots.kind_of(&path).is_some() {
                        pending.insert(path);
                        first.get_or_insert_with(Instant::now);
                        last = Instant::now();
                    }
                }
            }
            Some(Input::Event(Err(err))) => tracing::debug!("CLI usage watcher: {err}"),
            None => {
                first = None;
                on_batch(std::mem::take(&mut pending).into_iter().collect());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_dir::TestDir;

    /// The folder appears after the watch started: its files arrive without polling.
    #[test]
    fn a_root_created_later_is_picked_up() {
        let dir = TestDir::new("cli-watch-late");
        let home = dir.path().join("home");
        std::fs::create_dir_all(&home).expect("home");
        let roots = CliRoots {
            claude: Some(home.join(".claude/projects")),
            ..CliRoots::default()
        };
        let (tx, rx) = mpsc::channel();
        let _watch = start(&roots, Pace::LIVE, move |batch| {
            let _ = tx.send(batch);
        })
        .expect("watching the home folder");

        let session = home.join(".claude/projects/-work/s1.jsonl");
        std::fs::create_dir_all(session.parent().expect("parent")).expect("mkdir");
        std::fs::write(&session, "{}\n").expect("write");
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut seen = Vec::new();
        while Instant::now() < deadline && !seen.contains(&session) {
            if let Ok(batch) = rx.recv_timeout(Duration::from_millis(200)) {
                seen.extend(batch);
            }
        }
        assert!(seen.contains(&session), "got {seen:?}");
    }
}
