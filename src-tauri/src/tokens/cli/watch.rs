//! Watches the CLI session folders with `notify` (no polling) and hands over the changed
//! session files in batches, once writes pause for `QUIET` (at most every `MAX_WAIT`).

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::CliRoots;

const QUIET: Duration = Duration::from_millis(500);
const MAX_WAIT: Duration = Duration::from_secs(3);

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

/// Starts watching every existing root. `on_batch` runs on the watcher thread.
pub fn start(
    roots: &CliRoots,
    on_batch: impl FnMut(Vec<PathBuf>) + Send + 'static,
) -> Option<CliWatch> {
    let (input, events) = mpsc::channel();
    let forward = input.clone();
    let mut watcher = match notify::recommended_watcher(move |event| {
        let _ = forward.send(Input::Event(event));
    }) {
        Ok(watcher) => watcher,
        Err(err) => {
            tracing::warn!("cannot watch CLI sessions: {err}");
            return None;
        }
    };
    let mut watching = 0;
    for root in [&roots.claude, &roots.codex, &roots.gemini]
        .into_iter()
        .flatten()
    {
        if !root.is_dir() {
            continue;
        }
        match watcher.watch(root, RecursiveMode::Recursive) {
            Ok(()) => watching += 1,
            Err(err) => tracing::warn!("not watching {}: {err}", root.display()),
        }
    }
    if watching == 0 {
        return None;
    }
    let roots = roots.clone();
    let spawned = std::thread::Builder::new()
        .name("agentflow-cli-usage".into())
        .spawn(move || run(watcher, &events, &roots, on_batch));
    match spawned {
        Ok(_) => Some(CliWatch { input }),
        Err(err) => {
            tracing::warn!("cannot start the CLI usage watcher: {err}");
            None
        }
    }
}

fn run(
    _watcher: RecommendedWatcher,
    events: &mpsc::Receiver<Input>,
    roots: &CliRoots,
    mut on_batch: impl FnMut(Vec<PathBuf>),
) {
    let mut pending: BTreeSet<PathBuf> = BTreeSet::new();
    let mut first: Option<Instant> = None;
    let mut last = Instant::now();
    loop {
        let input = match first {
            Some(first) => {
                let deadline = (last + QUIET).min(first + MAX_WAIT);
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
                if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                    for path in event.paths {
                        if roots.kind_of(&path).is_some() {
                            pending.insert(path);
                            first.get_or_insert_with(Instant::now);
                            last = Instant::now();
                        }
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
