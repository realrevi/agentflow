//! The `Provider` trait and the factory that turns a settings entry (plus its key from the
//! keychain) into an adapter: Anthropic Messages or OpenAI-compatible Chat Completions.

use std::future::Future;
use std::pin::Pin;

use super::anthropic::Anthropic;
use super::cancel::CancelToken;
use super::error::ProviderError;
use super::http::Http;
use super::keys::ApiKey;
use super::openai::{OpenAiCompatible, Preset};
use super::types::{ChatRequest, EventSink, ModelInfo, StreamEnd};
use super::url::parse_base_url;
use crate::settings::ProviderEntry;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// One configured AI provider.
pub trait Provider: Send + Sync {
    /// Streams one reply. Events reach `sink` as they arrive; the result says how the
    /// stream ended (or why it failed). Cancelling `cancel` stops it promptly.
    fn stream_chat<'a>(
        &'a self,
        http: &'a Http,
        chat: &'a ChatRequest,
        sink: &'a mut dyn EventSink,
        cancel: &'a CancelToken,
    ) -> BoxFuture<'a, Result<StreamEnd, ProviderError>>;

    /// The models this key can use, from the provider's models endpoint.
    fn list_models<'a>(
        &'a self,
        http: &'a Http,
    ) -> BoxFuture<'a, Result<Vec<ModelInfo>, ProviderError>>;
}

/// Builds the adapter for `entry`. Hosted providers need a key; local servers (Ollama,
/// a custom localhost URL) work without one.
pub fn build_provider(
    entry: &ProviderEntry,
    key: Option<ApiKey>,
) -> Result<Box<dyn Provider>, ProviderError> {
    let base = parse_base_url(&entry.base_url)?;
    let local = base.scheme() == "http";
    let needs_key = |preset: Option<Preset>| match preset {
        Some(Preset::Ollama) => false,
        Some(Preset::Custom) => !local,
        _ => true,
    };
    match entry.kind.as_str() {
        "anthropic" => {
            if key.is_none() && !local {
                return Err(missing_key(entry));
            }
            Ok(Box::new(Anthropic::new(base, key)))
        }
        "openai" => {
            let preset = Preset::parse(&entry.preset);
            if key.is_none() && needs_key(Some(preset)) {
                return Err(missing_key(entry));
            }
            Ok(Box::new(OpenAiCompatible::new(base, key, preset)))
        }
        other => Err(ProviderError::config(format!(
            "Unknown provider type \"{other}\"."
        ))),
    }
}

fn missing_key(entry: &ProviderEntry) -> ProviderError {
    let name = if entry.name.is_empty() {
        &entry.preset
    } else {
        &entry.name
    };
    ProviderError::config(format!(
        "{name} has no API key yet. Add one in Settings › Providers."
    ))
}
