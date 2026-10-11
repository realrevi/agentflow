//! Lazy listing: one folder level at a time, ignore rules applied, folders first.

use std::path::Path;

use ignore::DirEntry;

use super::model::FsEntry;
use super::paths::WorkspaceRoot;
use super::sort::sort_entries;
use super::walk::walker;
use crate::error::{AppError, AppResult};

pub fn list_dir(root: &WorkspaceRoot, dir: &Path, show_hidden: bool) -> AppResult<Vec<FsEntry>> {
    if !dir.is_dir() {
        return Err(AppError::InvalidInput(format!(
            "not a folder: {}",
            dir.display()
        )));
    }
    let mut entries = Vec::new();
    for result in walker(dir, show_hidden).max_depth(Some(1)).build() {
        match result {
            Ok(entry) if entry.depth() > 0 => entries.extend(to_entry(root, &entry)),
            Ok(_) => {}
            Err(err) => tracing::debug!("skipping unreadable entry: {err}"),
        }
    }
    sort_entries(&mut entries);
    Ok(entries)
}

/// `None` for names that are not valid UTF-8 and for symlinks leaving the workspace.
fn to_entry(root: &WorkspaceRoot, entry: &DirEntry) -> Option<FsEntry> {
    let path = entry.path();
    let file_type = entry.file_type()?;
    let is_symlink = file_type.is_symlink();
    let is_dir = if is_symlink {
        match std::fs::canonicalize(path) {
            Ok(target) if target.starts_with(root.path()) => target.is_dir(),
            Ok(_) => return None,
            // A broken link is listed as a file so it can still be renamed or deleted.
            Err(_) => false,
        }
    } else {
        file_type.is_dir()
    };
    Some(FsEntry {
        name: entry.file_name().to_str()?.to_owned(),
        path: root.relative(path)?,
        is_dir,
        is_symlink,
    })
}
