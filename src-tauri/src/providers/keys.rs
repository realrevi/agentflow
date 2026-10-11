//! API keys live ONLY in the OS keychain (Keychain, Credential Manager, Secret Service).
//! They are never written to settings, never logged, and never returned over IPC: the
//! frontend only learns `KeyStatus` (has a key + its last 4 characters).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::settings::is_valid_provider_id;
use crate::util::lock;

/// Keychain "service"; matches the app identifier in tauri.conf.json.
pub const KEYCHAIN_SERVICE: &str = "dev.agentflow.desktop";
const MAX_KEY_LEN: usize = 4096;
/// Shorter keys show no hint at all (4 characters would give away too much of them).
const MIN_LEN_FOR_HINT: usize = 12;

/// Where secrets are stored. The app uses the OS keychain; tests use `MemoryStore`.
pub trait SecretStore: Send + Sync {
    fn get(&self, account: &str) -> Result<Option<String>, String>;
    fn set(&self, account: &str, secret: &str) -> Result<(), String>;
    fn delete(&self, account: &str) -> Result<(), String>;
}

/// The platform keychain via the `keyring` crate.
pub struct OsKeychain;

impl OsKeychain {
    fn entry(account: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(KEYCHAIN_SERVICE, account).map_err(|err| err.to_string())
    }
}

impl SecretStore for OsKeychain {
    fn get(&self, account: &str) -> Result<Option<String>, String> {
        match Self::entry(account)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(err.to_string()),
        }
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), String> {
        Self::entry(account)?
            .set_password(secret)
            .map_err(|err| err.to_string())
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        match Self::entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }
}

/// In-memory store for tests.
#[cfg(test)]
#[derive(Default)]
pub struct MemoryStore {
    secrets: Mutex<HashMap<String, String>>,
}

#[cfg(test)]
impl SecretStore for MemoryStore {
    fn get(&self, account: &str) -> Result<Option<String>, String> {
        Ok(lock(&self.secrets).get(account).cloned())
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), String> {
        lock(&self.secrets).insert(account.to_owned(), secret.to_owned());
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        lock(&self.secrets).remove(account);
        Ok(())
    }
}

/// An API key in memory. `Debug` never prints it.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiKey(***)")
    }
}

/// All the frontend ever learns about a key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyStatus {
    pub has_key: bool,
    pub last4: Option<String>,
}

impl KeyStatus {
    fn of(key: Option<&ApiKey>) -> Self {
        let Some(key) = key else {
            return Self {
                has_key: false,
                last4: None,
            };
        };
        let chars: Vec<char> = key.expose().chars().collect();
        let last4 = (chars.len() >= MIN_LEN_FOR_HINT)
            .then(|| chars[chars.len() - 4..].iter().collect::<String>());
        Self {
            has_key: true,
            last4,
        }
    }
}

/// Managed state: the keychain plus a small in-memory cache (one keychain read per key
/// and app run). Calls block on the OS keychain: run them via `run_blocking`.
#[derive(Clone)]
pub struct KeyVault {
    store: Arc<dyn SecretStore>,
    cache: Arc<Mutex<HashMap<String, Option<ApiKey>>>>,
}

impl KeyVault {
    pub fn new(store: Arc<dyn SecretStore>) -> Self {
        Self {
            store,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn status(&self, provider_id: &str) -> AppResult<KeyStatus> {
        Ok(KeyStatus::of(self.get(provider_id)?.as_ref()))
    }

    /// Stores a key (trimmed). Returns the new status, never the key.
    pub fn set(&self, provider_id: &str, key: &str) -> AppResult<KeyStatus> {
        let account = account(provider_id)?;
        let key = key.trim();
        if key.is_empty() || key.len() > MAX_KEY_LEN || key.chars().any(char::is_whitespace) {
            return Err(AppError::InvalidInput(
                "an API key is one line of text without spaces".into(),
            ));
        }
        self.store
            .set(&account, key)
            .map_err(|err| keychain_error("save the key", &err))?;
        let key = ApiKey(key.to_owned());
        let status = KeyStatus::of(Some(&key));
        lock(&self.cache).insert(account, Some(key));
        Ok(status)
    }

    pub fn delete(&self, provider_id: &str) -> AppResult<()> {
        let account = account(provider_id)?;
        self.store
            .delete(&account)
            .map_err(|err| keychain_error("remove the key", &err))?;
        lock(&self.cache).insert(account, None);
        Ok(())
    }

    /// The key itself, for building requests inside the backend only.
    pub(crate) fn get(&self, provider_id: &str) -> AppResult<Option<ApiKey>> {
        let account = account(provider_id)?;
        if let Some(cached) = lock(&self.cache).get(&account) {
            return Ok(cached.clone());
        }
        let key = self
            .store
            .get(&account)
            .map_err(|err| keychain_error("read the key", &err))?
            .map(ApiKey);
        lock(&self.cache).insert(account, key.clone());
        Ok(key)
    }
}

fn account(provider_id: &str) -> AppResult<String> {
    if !is_valid_provider_id(provider_id) {
        return Err(AppError::InvalidInput(format!(
            "bad provider id: {provider_id}"
        )));
    }
    Ok(format!("provider:{provider_id}"))
}

/// Keychain errors never contain the secret, only the store's own message.
fn keychain_error(action: &str, detail: &str) -> AppError {
    AppError::Internal(format!("could not {action} in the OS keychain: {detail}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault() -> (KeyVault, Arc<MemoryStore>) {
        let store = Arc::new(MemoryStore::default());
        (KeyVault::new(store.clone()), store)
    }

    #[test]
    fn stores_keys_in_the_keychain_only() {
        let (vault, store) = vault();
        assert_eq!(
            vault.status("p1").expect("status"),
            KeyStatus {
                has_key: false,
                last4: None
            }
        );
        let status = vault.set("p1", "  sk-test-0123456789abcd  ").expect("set");
        assert_eq!(status.last4.as_deref(), Some("abcd"));
        assert!(status.has_key);
        assert_eq!(
            store.get("provider:p1").expect("get").as_deref(),
            Some("sk-test-0123456789abcd")
        );
        assert_eq!(
            vault.get("p1").expect("get").map(|k| k.expose().to_owned()),
            Some("sk-test-0123456789abcd".to_owned())
        );
        vault.delete("p1").expect("delete");
        assert!(!vault.status("p1").expect("status").has_key);
        assert_eq!(store.get("provider:p1").expect("get"), None);
    }

    #[test]
    fn short_keys_give_no_hint_and_bad_input_is_refused() {
        let (vault, _) = vault();
        let status = vault.set("p1", "abc123").expect("set");
        assert!(status.has_key);
        assert_eq!(status.last4, None);
        assert!(vault.set("p1", "two words").is_err());
        assert!(vault.set("p1", "   ").is_err());
        assert!(vault.set("../evil", "sk-0123456789abcdef").is_err());
    }

    #[test]
    fn reads_the_keychain_once_then_uses_the_cache() {
        let (vault, store) = vault();
        store
            .set("provider:p2", "sk-from-keychain-12345")
            .expect("seed");
        assert_eq!(
            vault.status("p2").expect("status").last4.as_deref(),
            Some("2345")
        );
        store
            .delete("provider:p2")
            .expect("remove behind the vault's back");
        assert!(vault.status("p2").expect("cached").has_key);
    }

    #[test]
    fn debug_output_never_shows_the_key() {
        let key = ApiKey("sk-secret-value-123456".into());
        assert_eq!(format!("{key:?}"), "ApiKey(***)");
        let status = KeyStatus::of(Some(&key));
        let json = serde_json::to_string(&status).expect("json");
        assert!(!json.contains("secret-value"), "{json}");
    }
}
