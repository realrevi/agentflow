//! The workspace file system: lazy listing, read/write with conflict detection, create,
//! rename, delete to trash, fuzzy quick open and a debounced watcher. Every path from
//! the frontend is relative to the workspace root and is checked to stay inside it.

mod debounce;
mod filter;
mod fuzzy;
mod index;
mod list;
mod model;
mod paths;
mod read;
mod sort;
mod walk;
mod watch;
mod workspace;
mod write;

#[cfg(test)]
mod tests;

pub use model::{EntryKind, FileContent, FileStat, FsChanges, FsEntry, QuickOpenMatch};
pub use workspace::{FsState, Workspace};

// For the agent tools, which share the sandbox and the limits.
pub(crate) use paths::{relative_to, WorkspaceRoot};
pub(crate) use read::read_file;
pub(crate) use walk::walker;
pub(crate) use write::{move_to_trash, write_file};
