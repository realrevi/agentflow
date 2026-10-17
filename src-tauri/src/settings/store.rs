//! Managed Tauri state holding the current settings and serializing writes.

use std::path::PathBuf;

use tokio::sync::Mutex;

use super::io::{load_from_file, save_to_file};
use super::model::Settings;
use crate::error::AppResult;
use crate::util::run_blocking;

pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsStore {
    pub fn load(path: PathBuf) -> Self {
        let current = Mutex::new(load_from_file(&path));
        Self { path, current }
    }

    /// The current settings without awaiting (app setup, before anything else holds the lock).
    pub fn snapshot(&self) -> Settings {
        match self.current.try_lock() {
            Ok(current) => current.clone(),
            Err(_) => load_from_file(&self.path),
        }
    }

    pub async fn get(&self) -> Settings {
        self.current.lock().await.clone()
    }

    /// Normalizes, persists and returns the settings that were actually stored.
    /// The lock is held across the write so concurrent saves cannot interleave.
    pub async fn save(&self, next: Settings) -> AppResult<Settings> {
        let next = next.normalized();
        let mut current = self.current.lock().await;
        let path = self.path.clone();
        let to_write = next.clone();
        run_blocking(move || save_to_file(&path, &to_write)).await?;
        *current = next.clone();
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_dir::TestDir;

    #[test]
    fn save_normalizes_and_persists() {
        let dir = TestDir::new("settings-store");
        let path = dir.path().join("settings.json");
        let store = SettingsStore::load(path.clone());

        let request = Settings {
            language: "klingon".into(),
            ..Settings::default()
        };
        let saved = tauri::async_runtime::block_on(store.save(request)).expect("save");

        assert_eq!(saved.language, "en");
        assert_eq!(tauri::async_runtime::block_on(store.get()), saved);
        assert_eq!(SettingsStore::load(path).current.into_inner(), saved);
    }
}
