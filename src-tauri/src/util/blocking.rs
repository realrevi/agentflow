//! Runs blocking file I/O off the async runtime so commands never stall the UI thread.

use crate::error::{AppError, AppResult};

pub async fn run_blocking<T, F>(task: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|err| AppError::Internal(format!("background task failed: {err}")))?
}
