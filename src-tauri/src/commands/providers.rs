//! Provider setup: keys (write-only: the frontend only ever gets a `KeyStatus`), model
//! lists and "Test connection".

use serde::Serialize;
use tauri::State;

use crate::chat::{tiny_request, ChatManager};
use crate::error::{AppError, AppResult};
use crate::providers::{
    build_provider, ErrorKind, EventSink, KeyStatus, KeyVault, ModelInfo, Provider, StreamEnd,
    StreamEvent,
};
use crate::settings::{ProviderEntry, SettingsStore};
use crate::tokens::UsageTracker;
use crate::util::run_blocking;

#[tauri::command]
pub async fn provider_key_status(
    vault: State<'_, KeyVault>,
    provider_id: String,
) -> AppResult<KeyStatus> {
    let vault = vault.inner().clone();
    run_blocking(move || vault.status(&provider_id)).await
}

/// Saves the key in the OS keychain. Returns the status, never the key.
#[tauri::command]
pub async fn provider_set_key(
    vault: State<'_, KeyVault>,
    provider_id: String,
    key: String,
) -> AppResult<KeyStatus> {
    let vault = vault.inner().clone();
    run_blocking(move || vault.set(&provider_id, &key)).await
}

#[tauri::command]
pub async fn provider_delete_key(vault: State<'_, KeyVault>, provider_id: String) -> AppResult<()> {
    let vault = vault.inner().clone();
    run_blocking(move || vault.delete(&provider_id)).await
}

/// Lists models; their context windows (and OpenRouter's prices) go to the usage tracker.
#[tauri::command]
pub async fn provider_models(
    settings: State<'_, SettingsStore>,
    vault: State<'_, KeyVault>,
    manager: State<'_, ChatManager>,
    tracker: State<'_, UsageTracker>,
    provider_id: String,
) -> AppResult<Vec<ModelInfo>> {
    let entry = entry(&settings, &provider_id).await?;
    let (provider, secret) = provider_for(&settings, &vault, &provider_id).await?;
    let http = manager.http()?;
    let models = provider
        .list_models(&http)
        .await
        .map_err(|err| AppError::Provider(err.redacted(secret.as_deref()).message))?;
    tracker.note_models(&entry.id, &entry.preset, &models);
    Ok(models)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTest {
    pub models: usize,
    /// `models` (listed them) or `request` (no model list; a tiny request worked).
    pub method: &'static str,
}

/// Lists models; if the provider has no models endpoint, sends a tiny request instead.
#[tauri::command]
pub async fn provider_test(
    settings: State<'_, SettingsStore>,
    vault: State<'_, KeyVault>,
    manager: State<'_, ChatManager>,
    provider_id: String,
) -> AppResult<ConnectionTest> {
    let entry = entry(&settings, &provider_id).await?;
    let (provider, secret) = provider_for(&settings, &vault, &provider_id).await?;
    let http = manager.http()?;
    let fail = |err: crate::providers::ProviderError| {
        AppError::Provider(err.redacted(secret.as_deref()).message)
    };
    match provider.list_models(&http).await {
        Ok(models) if !models.is_empty() => {
            return Ok(ConnectionTest {
                models: models.len(),
                method: "models",
            })
        }
        Ok(_) => {}
        Err(err) if matches!(err.kind, ErrorKind::InvalidModel | ErrorKind::BadRequest) => {}
        Err(err) => return Err(fail(err)),
    }
    if entry.default_model.is_empty() {
        return Err(AppError::Provider(
            "The provider lists no models: set a default model to test with a short request."
                .into(),
        ));
    }
    let (_handle, cancel) = crate::providers::cancel_pair();
    let chat = tiny_request(&entry.default_model);
    match provider
        .stream_chat(&http, &chat, &mut Discard, &cancel)
        .await
    {
        Ok(StreamEnd::Stopped(_) | StreamEnd::Cancelled) => Ok(ConnectionTest {
            models: 0,
            method: "request",
        }),
        Err(err) => Err(fail(err)),
    }
}

struct Discard;

impl EventSink for Discard {
    fn event(&mut self, _event: StreamEvent) {}
}

async fn entry(settings: &SettingsStore, provider_id: &str) -> AppResult<ProviderEntry> {
    settings
        .get()
        .await
        .providers
        .into_iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| AppError::NotFound(format!("provider {provider_id}")))
}

async fn provider_for(
    settings: &SettingsStore,
    vault: &KeyVault,
    provider_id: &str,
) -> AppResult<(Box<dyn Provider>, Option<String>)> {
    let entry = entry(settings, provider_id).await?;
    let key = {
        let (vault, id) = (vault.clone(), provider_id.to_owned());
        run_blocking(move || vault.get(&id)).await?
    };
    let secret = key.as_ref().map(|k| k.expose().to_owned());
    let provider = build_provider(&entry, key)?;
    Ok((provider, secret))
}
