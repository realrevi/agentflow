use tauri::State;

use crate::error::AppResult;
use crate::themes::{ThemeData, ThemeStore, ThemeSummary};
use crate::util::run_blocking;

#[tauri::command]
pub async fn list_themes(store: State<'_, ThemeStore>) -> AppResult<Vec<ThemeSummary>> {
    let store = store.inner().clone();
    run_blocking(move || Ok(store.list())).await
}

#[tauri::command]
pub async fn get_theme(store: State<'_, ThemeStore>, id: String) -> AppResult<ThemeData> {
    let store = store.inner().clone();
    run_blocking(move || store.get(&id)).await
}

/// The frontend reads the picked file and sends its text, so no dialog/fs plugin is needed.
#[tauri::command]
pub async fn import_theme(
    store: State<'_, ThemeStore>,
    file_name: String,
    contents: String,
) -> AppResult<ThemeSummary> {
    let store = store.inner().clone();
    run_blocking(move || store.import(&file_name, &contents)).await
}
