//! Watches the workspace with `notify` and streams debounced, filtered batches.
//!
//! macOS and Windows watch the root recursively (cheap, native). On Linux every
//! inotify watch costs kernel memory, so only visible folders are watched one by one
//! (never `node_modules`, `target` or `.git`) and new folders are added as they appear.

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::Instant;

use notify::event::{EventKind, ModifyKind};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use super::debounce::{Debouncer, MAX_WAIT, QUIET};
use super::filter::EventFilter;
use super::index::FileIndex;
use super::model::FsChanges;
use super::walk::{is_ignore_file, walker};
use crate::error::{AppError, AppResult};

const PER_FOLDER_WATCHES: bool = cfg!(target_os = "linux");

enum Input {
    Event(notify::Result<Event>),
    Stop,
}

/// Stops the watcher thread when dropped.
pub struct WatchHandle {
    input: mpsc::Sender<Input>,
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        let _ = self.input.send(Input::Stop);
    }
}

pub fn start(
    root: PathBuf,
    show_hidden: bool,
    index: Arc<FileIndex>,
    sink: impl FnMut(FsChanges) + Send + 'static,
) -> AppResult<WatchHandle> {
    let (input, events) = mpsc::channel();
    let forward = input.clone();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = forward.send(Input::Event(event));
    })
    .map_err(watch_error)?;
    if PER_FOLDER_WATCHES {
        watch_tree(&mut watcher, &root, show_hidden);
    } else {
        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(watch_error)?;
    }
    let state = WatchState {
        filter: EventFilter::new(root.clone(), show_hidden),
        debouncer: Debouncer::new(QUIET, MAX_WAIT),
        root,
        show_hidden,
        index,
        watcher,
    };
    std::thread::Builder::new()
        .name("agentflow-fs-watch".into())
        .spawn(move || state.run(&events, sink))
        .map_err(|err| AppError::Internal(format!("could not start the file watcher: {err}")))?;
    Ok(WatchHandle { input })
}

struct WatchState {
    filter: EventFilter,
    debouncer: Debouncer,
    root: PathBuf,
    show_hidden: bool,
    index: Arc<FileIndex>,
    /// Owned by this thread; dropping it stops notify's own thread.
    watcher: RecommendedWatcher,
}

impl WatchState {
    fn run(mut self, events: &mpsc::Receiver<Input>, mut sink: impl FnMut(FsChanges)) {
        loop {
            let input = match self.debouncer.deadline() {
                Some(deadline) => {
                    match events.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                        Ok(input) => Some(input),
                        Err(mpsc::RecvTimeoutError::Timeout) => None,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
                None => match events.recv() {
                    Ok(input) => Some(input),
                    Err(_) => break,
                },
            };
            match input {
                Some(Input::Stop) => break,
                Some(Input::Event(Ok(event))) => self.handle(event),
                Some(Input::Event(Err(err))) => {
                    tracing::warn!("file watcher error: {err}");
                    self.debouncer.rescan(Instant::now());
                }
                None => {}
            }
            if let Some(changes) = self.debouncer.take_if_due(Instant::now()) {
                sink(changes);
            }
        }
        tracing::debug!("file watcher stopped for {}", self.root.display());
    }

    fn handle(&mut self, event: Event) {
        let now = Instant::now();
        if event.need_rescan() {
            self.index.invalidate();
            self.debouncer.rescan(now);
            return;
        }
        let structural = match event.kind {
            EventKind::Access(_) => return,
            EventKind::Modify(ModifyKind::Name(_)) => true,
            EventKind::Modify(_) => false,
            _ => true,
        };
        for path in &event.paths {
            if path.file_name().is_some_and(is_ignore_file) {
                // What is visible may have changed anywhere: reload rules, re-list all.
                self.filter.reload();
                self.index.invalidate();
                self.debouncer.rescan(now);
            }
            let Some(rel) = self.filter.visible(path) else {
                continue;
            };
            if structural {
                self.index.invalidate();
                if PER_FOLDER_WATCHES && matches!(event.kind, EventKind::Create(_)) {
                    self.watch_new_folder(path);
                }
            }
            self.debouncer.add(rel, structural, now);
        }
    }

    fn watch_new_folder(&mut self, path: &Path) {
        let is_folder = std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir());
        if is_folder {
            watch_tree(&mut self.watcher, path, self.show_hidden);
        }
    }
}

/// Watches `dir` and every visible folder below it, one non-recursive watch each.
fn watch_tree(watcher: &mut RecommendedWatcher, dir: &Path, show_hidden: bool) {
    let folders = walker(dir, show_hidden)
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_dir()));
    for folder in folders {
        if let Err(err) = watcher.watch(folder.path(), RecursiveMode::NonRecursive) {
            // Usually the inotify watch limit; the tree still works, just not live there.
            tracing::warn!("not watching {}: {err}", folder.path().display());
            return;
        }
    }
}

fn watch_error(err: notify::Error) -> AppError {
    AppError::Internal(format!("could not watch the workspace: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_dir::TestDir;
    use std::fs;
    use std::time::Duration;

    #[test]
    fn real_changes_arrive_as_one_filtered_batch() {
        let dir = TestDir::new("fs-watch");
        let root = dir.path().canonicalize().expect("canonical");
        fs::write(root.join(".gitignore"), "*.log\n").expect("write");
        fs::create_dir_all(root.join("src")).expect("mkdir");
        let index = Arc::new(FileIndex::new(root.clone(), false));
        let (sender, batches) = mpsc::channel();
        let handle = start(root.clone(), false, index, move |changes| {
            let _ = sender.send(changes);
        })
        .expect("start");

        fs::write(root.join("src/a.rs"), "a").expect("write");
        fs::write(root.join("src/b.rs"), "b").expect("write");
        fs::write(root.join("debug.log"), "ignored").expect("write");
        fs::create_dir(root.join("src/new")).expect("mkdir");

        let batch = batches
            .recv_timeout(Duration::from_secs(5))
            .expect("a batch");
        for path in ["src/a.rs", "src/b.rs", "src/new"] {
            assert!(
                batch.structural.iter().any(|p| p == path),
                "{path} in {batch:?}"
            );
        }
        assert!(
            !batch.paths.iter().any(|p| p.ends_with(".log")),
            "ignored: {batch:?}"
        );

        // Folders created after start are watched too (per-folder watches on Linux).
        std::thread::sleep(Duration::from_millis(100));
        fs::write(root.join("src/new/c.rs"), "c").expect("write");
        let later = batches
            .recv_timeout(Duration::from_secs(5))
            .expect("second batch");
        assert!(later.paths.iter().any(|p| p == "src/new/c.rs"), "{later:?}");
        drop(handle);
    }
}
