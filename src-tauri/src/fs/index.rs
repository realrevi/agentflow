//! Quick open: a cached list of every visible file, built with the parallel walker and
//! dropped whenever the watcher reports a structural change.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::{mpsc, Arc, Mutex};

use ignore::WalkState;

use super::fuzzy::Query;
use super::model::QuickOpenMatch;
use super::paths::relative_to;
use super::sort::natural_cmp;
use super::walk::walker;
use crate::util::lock;

/// Bounds memory on huge trees (~100 bytes per path).
const MAX_FILES: usize = 100_000;

pub struct FileIndex {
    root: PathBuf,
    show_hidden: bool,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    /// Bumped on every invalidation, so a build that raced with one is not cached.
    generation: u64,
    files: Option<Arc<Vec<String>>>,
}

impl FileIndex {
    pub fn new(root: PathBuf, show_hidden: bool) -> Self {
        Self {
            root,
            show_hidden,
            state: Mutex::new(State::default()),
        }
    }

    pub fn invalidate(&self) {
        let mut state = lock(&self.state);
        state.generation += 1;
        state.files = None;
    }

    /// Best matches first; an empty query lists files closest to the root.
    pub fn query(&self, text: &str, limit: usize) -> Vec<QuickOpenMatch> {
        let files = self.files();
        let query = Query::new(text);
        if query.is_empty() {
            return files
                .iter()
                .take(limit)
                .map(|path| QuickOpenMatch {
                    path: path.clone(),
                    positions: Vec::new(),
                })
                .collect();
        }
        let mut scored: Vec<(i64, usize, Vec<u32>)> = files
            .iter()
            .enumerate()
            .filter_map(|(i, path)| query.score(path).map(|(score, at)| (score, i, at)))
            .collect();
        let order = |a: &(i64, usize, Vec<u32>), b: &(i64, usize, Vec<u32>)| -> Ordering {
            b.0.cmp(&a.0)
                .then_with(|| files[a.1].len().cmp(&files[b.1].len()))
                .then_with(|| a.1.cmp(&b.1))
        };
        if scored.len() > limit {
            scored.select_nth_unstable_by(limit, order);
            scored.truncate(limit);
        }
        scored.sort_unstable_by(order);
        scored
            .into_iter()
            .map(|(_, i, positions)| QuickOpenMatch {
                path: files[i].clone(),
                positions,
            })
            .collect()
    }

    fn files(&self) -> Arc<Vec<String>> {
        let generation = {
            let state = lock(&self.state);
            if let Some(files) = &state.files {
                return Arc::clone(files);
            }
            state.generation
        };
        let files = Arc::new(collect_files(&self.root, self.show_hidden));
        let mut state = lock(&self.state);
        if state.generation == generation {
            state.files = Some(Arc::clone(&files));
        }
        files
    }
}

/// Every visible file as a relative path, shallow paths first.
fn collect_files(root: &Path, show_hidden: bool) -> Vec<String> {
    let (sender, receiver) = mpsc::channel::<String>();
    let count = AtomicUsize::new(0);
    walker(root, show_hidden).build_parallel().run(|| {
        let sender = sender.clone();
        let count = &count;
        Box::new(move |result| {
            let Ok(entry) = result else {
                return WalkState::Continue;
            };
            if !is_file_inside(root, entry.path(), entry.file_type()) {
                return WalkState::Continue;
            }
            if count.fetch_add(1, AtomicOrdering::Relaxed) >= MAX_FILES {
                return WalkState::Quit;
            }
            if let Some(rel) = relative_to(root, entry.path()) {
                let _ = sender.send(rel);
            }
            WalkState::Continue
        })
    });
    drop(sender);
    let mut files: Vec<String> = receiver.into_iter().collect();
    if files.len() >= MAX_FILES {
        tracing::warn!("quick open indexed only the first {MAX_FILES} files");
    }
    files.sort_by(|a, b| {
        let depth = |path: &str| path.matches('/').count();
        depth(a).cmp(&depth(b)).then_with(|| natural_cmp(a, b))
    });
    files
}

fn is_file_inside(root: &Path, path: &Path, file_type: Option<std::fs::FileType>) -> bool {
    match file_type {
        Some(kind) if kind.is_file() => true,
        // Symlinked files are listed when they point inside the workspace.
        Some(kind) if kind.is_symlink() => std::fs::canonicalize(path)
            .is_ok_and(|target| target.starts_with(root) && target.is_file()),
        _ => false,
    }
}
