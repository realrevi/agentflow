//! App updates (phase 12): check for a newer release, then install it on request.

use tauri::ipc::Channel;
use tauri::{AppHandle, State};

use crate::error::AppResult;
use crate::updates::{UpdateInfo, UpdateProgress, Updates};

/// The newer release, or `None` when this version is the latest (or nothing is published).
#[tauri::command]
pub async fn update_check(
    app: AppHandle,
    updates: State<'_, Updates>,
) -> AppResult<Option<UpdateInfo>> {
    updates.check(&app).await
}

/// Downloads, verifies and installs the update from the last check, then restarts the app.
#[tauri::command]
pub async fn update_install(
    app: AppHandle,
    updates: State<'_, Updates>,
    on_event: Channel<UpdateProgress>,
) -> AppResult<()> {
    updates.install(&app, on_event).await
}
