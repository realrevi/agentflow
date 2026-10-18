//! The community agent library (phase 10): the cached catalog, refreshes (forced, or at
//! most once a day), one item in full for the detail view and the wizard, and the optional
//! GitHub token (OS keychain only; the frontend gets a `KeyStatus`).

use tauri::State;

use crate::chat::ChatManager;
use crate::error::AppResult;
use crate::library::{Catalog, ItemDetail, Library};
use crate::providers::{KeyStatus, KeyVault};
use crate::settings::SettingsStore;
use crate::util::run_blocking;

async fn token(vault: &KeyVault) -> AppResult<Option<String>> {
    let vault = vault.clone();
    let token = run_blocking(move || vault.github()).await?;
    Ok(token.map(|t| t.expose().to_owned()))
}

/// The catalog as cached (works offline).
#[tauri::command]
pub async fn library_catalog(library: State<'_, Library>) -> AppResult<Catalog> {
    let library = library.inner().clone();
    run_blocking(move || Ok(library.catalog())).await
}

/// `force: false` only refreshes when the last refresh is over a day old.
#[tauri::command]
pub async fn library_refresh(
    library: State<'_, Library>,
    settings: State<'_, SettingsStore>,
    vault: State<'_, KeyVault>,
    manager: State<'_, ChatManager>,
    force: bool,
) -> AppResult<Catalog> {
    let custom = settings.snapshot().library.custom_sources;
    let token = token(&vault).await?;
    let http = manager.http()?;
    library.refresh(http, token, &custom, force).await
}

#[tauri::command]
pub async fn library_detail(
    library: State<'_, Library>,
    vault: State<'_, KeyVault>,
    manager: State<'_, ChatManager>,
    id: String,
) -> AppResult<ItemDetail> {
    let token = token(&vault).await?;
    let http = manager.http()?;
    library.detail(http, token, &id).await
}

#[tauri::command]
pub async fn library_token_status(vault: State<'_, KeyVault>) -> AppResult<KeyStatus> {
    let vault = vault.inner().clone();
    run_blocking(move || vault.github_status()).await
}

#[tauri::command]
pub async fn library_set_token(vault: State<'_, KeyVault>, token: String) -> AppResult<KeyStatus> {
    let vault = vault.inner().clone();
    run_blocking(move || vault.set_github(&token)).await
}

#[tauri::command]
pub async fn library_delete_token(vault: State<'_, KeyVault>) -> AppResult<()> {
    let vault = vault.inner().clone();
    run_blocking(move || vault.delete_github()).await
}
