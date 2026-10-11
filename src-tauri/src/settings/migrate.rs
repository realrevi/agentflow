//! Upgrades raw settings JSON from any older `version` to `CURRENT_VERSION`.

use serde_json::{Map, Value};

use super::model::{Settings, CURRENT_VERSION};
use crate::error::AppResult;

/// `STEPS[n]` upgrades a document from version `n` to `n + 1`.
const STEPS: [fn(&mut Map<String, Value>); CURRENT_VERSION as usize] = [v0_to_v1, v1_to_v2];

/// Migrates and deserializes a raw settings document. Missing fields get defaults,
/// unknown fields are dropped, and invalid values are repaired by `normalized()`.
pub fn migrate(raw: Value) -> AppResult<Settings> {
    let mut doc = match raw {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    let from = doc.get("version").and_then(Value::as_u64).unwrap_or(0);

    if from > u64::from(CURRENT_VERSION) {
        tracing::warn!(
            "settings version {from} is newer than {CURRENT_VERSION}; loading known fields only"
        );
    } else {
        // `from` is at most CURRENT_VERSION here, so the cast and slice are in range.
        for step in &STEPS[from as usize..] {
            step(&mut doc);
        }
    }

    let settings: Settings = serde_json::from_value(Value::Object(doc))?;
    Ok(settings.normalized())
}

/// v0 = files written before settings were versioned. Same shape, no `version` field.
fn v0_to_v1(doc: &mut Map<String, Value>) {
    doc.insert("version".into(), Value::from(1));
}

/// v2 adds the workspace folder and terminal settings (their defaults fill in on load).
/// The saved layout is dropped: v1 layouts hold "agent" placeholder panels that are now
/// real terminal panels with a different shape, so the default layout is rebuilt.
fn v1_to_v2(doc: &mut Map<String, Value>) {
    doc.insert("version".into(), Value::from(2));
    doc.insert("layout".into(), Value::Null);
    doc.entry("currentWorkspace").or_insert(Value::Null);
    doc.entry("recentWorkspaces")
        .or_insert_with(|| Value::Array(Vec::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::TerminalSettings;
    use serde_json::json;

    #[test]
    fn unversioned_document_is_upgraded_with_defaults() {
        let settings = migrate(json!({ "language": "tr" })).expect("migrate");
        assert_eq!(settings.version, CURRENT_VERSION);
        assert_eq!(settings.language, "tr");
        assert_eq!(settings.appearance, Settings::default().appearance);
    }

    #[test]
    fn non_object_document_becomes_defaults() {
        assert_eq!(
            migrate(json!("oops")).expect("migrate"),
            Settings::default()
        );
    }

    #[test]
    fn current_document_round_trips() {
        let mut original = Settings {
            language: "de".into(),
            ..Settings::default()
        };
        original.appearance.agent_bold.glow = false;
        let raw = serde_json::to_value(&original).expect("serialize");
        assert_eq!(migrate(raw).expect("migrate"), original);
    }

    #[test]
    fn newer_version_keeps_known_fields() {
        let settings =
            migrate(json!({ "version": 99, "language": "ru", "future": true })).expect("migrate");
        assert_eq!(settings.language, "ru");
        assert_eq!(settings.version, CURRENT_VERSION);
    }

    #[test]
    fn v1_document_gains_workspace_and_terminal_settings() {
        let v1 = json!({
            "version": 1,
            "language": "zh",
            "appearance": { "themeId": "retro-crt" },
            "layout": { "grid": { "root": {} }, "panels": { "agent-1": {} } }
        });
        let settings = migrate(v1).expect("migrate");
        assert_eq!(settings.version, 2);
        assert_eq!(settings.language, "zh");
        assert_eq!(settings.appearance.theme_id, "retro-crt");
        assert_eq!(settings.layout, None, "phase 1 layouts are reset");
        assert_eq!(settings.current_workspace, None);
        assert!(settings.recent_workspaces.is_empty());
        assert_eq!(settings.terminal, TerminalSettings::default());
        assert_eq!(settings.terminal.font_size, 13);
        assert_eq!(settings.terminal.scrollback, 5000);
        assert!(settings.terminal.auto_start_shells);
    }

    #[test]
    fn v2_layout_is_kept() {
        let raw = json!({ "version": 2, "layout": { "grid": {} }, "currentWorkspace": "/src" });
        let settings = migrate(raw).expect("migrate");
        assert!(settings.layout.is_some());
        assert_eq!(settings.current_workspace.as_deref(), Some("/src"));
    }

    #[test]
    fn wrong_field_type_is_an_error() {
        assert!(migrate(json!({ "version": 1, "language": 42 })).is_err());
    }
}
