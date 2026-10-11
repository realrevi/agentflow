//! Reading and writing the settings file. Loading never fails: a missing file means
//! defaults, and an unreadable or corrupt file is backed up and replaced by defaults.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::migrate::migrate;
use super::model::Settings;
use crate::error::AppResult;
use crate::util::write_atomic;

pub fn load_from_file(path: &Path) -> Settings {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => return Settings::default(),
        Err(err) => {
            tracing::warn!("cannot read {}: {err}; using defaults", path.display());
            return Settings::default();
        }
    };

    let parsed = serde_json::from_str::<Value>(&text)
        .map_err(Into::into)
        .and_then(migrate);
    match parsed {
        Ok(settings) => settings,
        Err(err) => {
            tracing::warn!("corrupt settings file {}: {err}", path.display());
            back_up_corrupt(path);
            Settings::default()
        }
    }
}

pub fn save_to_file(path: &Path, settings: &Settings) -> AppResult<()> {
    let mut bytes = serde_json::to_vec_pretty(settings)?;
    bytes.push(b'\n');
    write_atomic(path, &bytes)?;
    Ok(())
}

/// Keeps the broken file next to the real one so user data is never silently lost.
fn back_up_corrupt(path: &Path) {
    let backup = backup_path(path);
    match fs::rename(path, &backup) {
        Ok(()) => tracing::warn!("moved corrupt settings to {}", backup.display()),
        Err(err) => tracing::warn!("could not back up corrupt settings: {err}"),
    }
}

fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".corrupt");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::CURRENT_VERSION;
    use crate::util::test_dir::TestDir;

    #[test]
    fn missing_file_loads_defaults() {
        let dir = TestDir::new("settings-missing");
        let settings = load_from_file(&dir.path().join("settings.json"));
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = TestDir::new("settings-roundtrip");
        let path = dir.path().join("settings.json");
        let settings = Settings {
            language: "ru".into(),
            layout: Some(serde_json::json!({ "grid": { "root": {} } })),
            ..Settings::default()
        };

        save_to_file(&path, &settings).expect("save");
        assert_eq!(load_from_file(&path), settings);

        let on_disk: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
        assert_eq!(on_disk["version"], CURRENT_VERSION);
        assert_eq!(on_disk["appearance"]["agentBold"]["color"], "#F5C542");
    }

    #[test]
    fn corrupt_file_is_backed_up_and_defaults_used() {
        let dir = TestDir::new("settings-corrupt");
        let path = dir.path().join("settings.json");
        fs::write(&path, "{ not json").expect("write");

        assert_eq!(load_from_file(&path), Settings::default());
        assert!(!path.exists());
        assert_eq!(
            fs::read_to_string(dir.path().join("settings.json.corrupt")).expect("backup"),
            "{ not json"
        );
    }

    #[test]
    fn unversioned_file_is_migrated() {
        let dir = TestDir::new("settings-v0");
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{ "language": "de", "ui": { "sidebarVisible": false } }"#,
        )
        .expect("write");

        let settings = load_from_file(&path);
        assert_eq!(settings.version, CURRENT_VERSION);
        assert_eq!(settings.language, "de");
        assert!(!settings.ui.sidebar_visible);
        assert_eq!(
            settings.ui.sidebar_width,
            Settings::default().ui.sidebar_width
        );
    }
}
