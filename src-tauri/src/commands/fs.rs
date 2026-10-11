use std::path::Path;

use tauri::ipc::Channel;
use tauri::State;

use crate::error::AppResult;
use crate::fs::{
    EntryKind, FileContent, FileStat, FsChanges, FsEntry, FsState, QuickOpenMatch, Workspace,
};
use crate::util::run_blocking;

/// Opens (or switches to) the workspace folder and starts watching it. Debounced change
/// batches arrive on `on_event` until the next open/close.
#[tauri::command]
pub async fn fs_open_workspace(
    state: State<'_, FsState>,
    root: String,
    show_hidden: bool,
    on_event: Channel<FsChanges>,
) -> AppResult<()> {
    let ticket = state.ticket();
    let workspace = run_blocking(move || {
        Workspace::open(Path::new(&root), show_hidden, move |changes| {
            // The frontend may be reloading; nothing to do then.
            let _ = on_event.send(changes);
        })
    })
    .await?;
    state.install(ticket, Some(workspace));
    Ok(())
}

#[tauri::command]
pub async fn fs_close_workspace(state: State<'_, FsState>) -> AppResult<()> {
    state.close();
    Ok(())
}

/// One folder level, ignore rules applied, folders first.
#[tauri::command]
pub async fn fs_list(state: State<'_, FsState>, dir: String) -> AppResult<Vec<FsEntry>> {
    let workspace = state.current()?;
    run_blocking(move || workspace.list(&dir)).await
}

#[tauri::command]
pub async fn fs_read(state: State<'_, FsState>, path: String) -> AppResult<FileContent> {
    let workspace = state.current()?;
    run_blocking(move || workspace.read(&path)).await
}

/// Atomic save. Fails with a `conflict` error if the file's mtime is no longer
/// `expected_mtime`; `None` writes unconditionally.
#[tauri::command]
pub async fn fs_write(
    state: State<'_, FsState>,
    path: String,
    content: String,
    expected_mtime: Option<u64>,
) -> AppResult<FileStat> {
    let workspace = state.current()?;
    run_blocking(move || workspace.write(&path, &content, expected_mtime)).await
}

#[tauri::command]
pub async fn fs_create(
    state: State<'_, FsState>,
    path: String,
    kind: EntryKind,
) -> AppResult<FsEntry> {
    let workspace = state.current()?;
    run_blocking(move || workspace.create(&path, kind)).await
}

#[tauri::command]
pub async fn fs_rename(state: State<'_, FsState>, from: String, to: String) -> AppResult<FsEntry> {
    let workspace = state.current()?;
    run_blocking(move || workspace.rename(&from, &to)).await
}

/// Moves to the OS trash; never deletes permanently.
#[tauri::command]
pub async fn fs_delete(state: State<'_, FsState>, path: String) -> AppResult<()> {
    let workspace = state.current()?;
    run_blocking(move || workspace.delete(&path)).await
}

/// Up to 50 fuzzy matches on relative paths.
#[tauri::command]
pub async fn fs_quick_open(
    state: State<'_, FsState>,
    query: String,
) -> AppResult<Vec<QuickOpenMatch>> {
    let workspace = state.current()?;
    run_blocking(move || Ok(workspace.quick_open(&query))).await
}
