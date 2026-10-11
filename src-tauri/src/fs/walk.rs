//! One `ignore` configuration for listing, quick open, the watcher and its event
//! filter, so they always agree on what is visible.

use std::ffi::OsStr;
use std::path::Path;

use ignore::WalkBuilder;

/// Always hidden, whatever the settings say.
pub const GIT_DIR: &str = ".git";

/// Respects `.gitignore` (even outside a git repository), `.ignore`, the global git
/// excludes and `.git/info/exclude`. Symlinks are not followed.
pub fn walker(dir: &Path, show_hidden: bool) -> WalkBuilder {
    let mut builder = WalkBuilder::new(dir);
    builder
        .hidden(!show_hidden)
        .ignore(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .parents(true)
        .require_git(false)
        .follow_links(false)
        .filter_entry(|entry| entry.file_name() != OsStr::new(GIT_DIR));
    builder
}

/// True for the files whose edits change what is ignored.
pub fn is_ignore_file(name: &OsStr) -> bool {
    name == OsStr::new(".gitignore") || name == OsStr::new(".ignore")
}
