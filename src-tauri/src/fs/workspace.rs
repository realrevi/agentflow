//! The open workspace (root, quick-open index, watcher) and the managed state holding it.

use std::path::Path;
use std::sync::{Arc, Mutex};

use super::index::FileIndex;
use super::list::list_dir;
use super::model::{EntryKind, FileContent, FileStat, FsChanges, FsEntry, QuickOpenMatch};
use super::paths::WorkspaceRoot;
use super::read::read_file;
use super::watch::{self, WatchHandle};
use super::write::{create, move_to_trash, rename, write_file};
use crate::error::{AppError, AppResult};
use crate::util::lock;

pub const QUICK_OPEN_LIMIT: usize = 50;

pub struct Workspace {
    root: WorkspaceRoot,
    show_hidden: bool,
    index: Arc<FileIndex>,
    /// Stops the watcher when the workspace is dropped. `None` if watching failed.
    _watch: Option<WatchHandle>,
}

impl Workspace {
    /// Opens `path` and starts watching it; `sink` receives debounced change batches.
    pub fn open(
        path: &Path,
        show_hidden: bool,
        sink: impl FnMut(FsChanges) + Send + 'static,
    ) -> AppResult<Self> {
        let root = WorkspaceRoot::open(path)?;
        let index = Arc::new(FileIndex::new(root.path().to_path_buf(), show_hidden));
        let watch = watch::start(
            root.path().to_path_buf(),
            show_hidden,
            Arc::clone(&index),
            sink,
        )
        .inspect_err(|err| tracing::warn!("{err}; the file tree will not update live"))
        .ok();
        Ok(Self {
            root,
            show_hidden,
            index,
            _watch: watch,
        })
    }

    pub fn list(&self, dir: &str) -> AppResult<Vec<FsEntry>> {
        list_dir(&self.root, &self.root.resolve(dir)?, self.show_hidden)
    }

    pub fn read(&self, path: &str) -> AppResult<FileContent> {
        read_file(&self.root.resolve(path)?)
    }

    pub fn write(&self, path: &str, content: &str, expected: Option<u64>) -> AppResult<FileStat> {
        // Existing files are written through symlinks (to their target), so links survive.
        let target = match self.root.resolve(path) {
            Err(AppError::NotFound(_)) => self.root.resolve_new(path)?,
            other => other?,
        };
        write_file(&target, content.as_bytes(), expected)
    }

    pub fn create(&self, path: &str, kind: EntryKind) -> AppResult<FsEntry> {
        let target = self.root.resolve_new(path)?;
        create(&target, kind)?;
        self.index.invalidate();
        self.entry(&target)
    }

    pub fn rename(&self, from: &str, to: &str) -> AppResult<FsEntry> {
        let source = self.root.resolve_entry(from)?;
        let target = self.root.resolve_new(to)?;
        rename(&source, &target)?;
        self.index.invalidate();
        self.entry(&target)
    }

    pub fn delete(&self, path: &str) -> AppResult<()> {
        move_to_trash(&self.root.resolve_entry(path)?)?;
        self.index.invalidate();
        Ok(())
    }

    pub fn quick_open(&self, query: &str) -> Vec<QuickOpenMatch> {
        self.index.query(query, QUICK_OPEN_LIMIT)
    }

    fn entry(&self, path: &Path) -> AppResult<FsEntry> {
        let meta = std::fs::symlink_metadata(path)?;
        let name = path.file_name().and_then(|name| name.to_str());
        let (Some(name), Some(rel)) = (name, self.root.relative(path)) else {
            return Err(AppError::InvalidInput(format!(
                "unsupported file name: {}",
                path.display()
            )));
        };
        Ok(FsEntry {
            name: name.to_owned(),
            path: rel,
            is_dir: path.is_dir(),
            is_symlink: meta.file_type().is_symlink(),
        })
    }
}

/// Managed Tauri state: the workspace the frontend currently shows.
#[derive(Default)]
pub struct FsState {
    inner: Mutex<Current>,
}

#[derive(Default)]
struct Current {
    workspace: Option<Arc<Workspace>>,
    /// Ticket of the open/close call that set `workspace`; older calls never win.
    ticket: u64,
    next_ticket: u64,
}

impl FsState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reserve a ticket before a slow `Workspace::open`, then `install` the result.
    pub fn ticket(&self) -> u64 {
        let mut inner = lock(&self.inner);
        inner.next_ticket += 1;
        inner.next_ticket
    }

    /// Replaces the workspace unless a later call already did. Returns whether it won.
    pub fn install(&self, ticket: u64, workspace: Option<Workspace>) -> bool {
        let mut inner = lock(&self.inner);
        if ticket < inner.ticket {
            return false;
        }
        inner.ticket = ticket;
        inner.workspace = workspace.map(Arc::new);
        true
    }

    pub fn current(&self) -> AppResult<Arc<Workspace>> {
        lock(&self.inner)
            .workspace
            .clone()
            .ok_or_else(|| AppError::NotFound("no workspace folder is open".into()))
    }

    /// Stops the watcher (app exit).
    pub fn close(&self) {
        let ticket = self.ticket();
        self.install(ticket, None);
    }
}
