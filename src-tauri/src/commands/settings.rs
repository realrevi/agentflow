use tauri::State;

use crate::error::AppResult;
use crate::settings::{Settings, SettingsStore};

#[tauri::command]
pub async fn get_settings(store: State<'_, SettingsStore>) -> AppResult<Settings> {
    Ok(store.get().await)
}

/// Returns the settings as stored (after normalization), which may differ from the input.
#[tauri::command]
pub async fn save_settings(
    store: State<'_, SettingsStore>,
    settings: Settings,
) -> AppResult<Settings> {
    store.save(settings).await
}
