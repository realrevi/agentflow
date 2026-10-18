//! Upgrades raw settings JSON from any older `version` to `CURRENT_VERSION`.

use serde_json::{Map, Value};

use super::model::{Settings, CURRENT_VERSION};
use crate::error::AppResult;

/// `STEPS[n]` upgrades a document from version `n` to `n + 1`.
const STEPS: [fn(&mut Map<String, Value>); CURRENT_VERSION as usize] = [
    v0_to_v1, v1_to_v2, v2_to_v3, v3_to_v4, v4_to_v5, v5_to_v6, v6_to_v7, v7_to_v8,
];

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

/// v3 adds file tree settings (defaults fill in on load). The layout is kept here: the
/// frontend versions its own layout format and replaces older layouts itself.
fn v2_to_v3(doc: &mut Map<String, Value>) {
    doc.insert("version".into(), Value::from(3));
}

/// v4 adds character settings (defaults fill in on load).
fn v3_to_v4(doc: &mut Map<String, Value>) {
    doc.insert("version".into(), Value::from(4));
}

/// v5 adds AI provider entries (none yet) and chat options (defaults fill in on load).
fn v4_to_v5(doc: &mut Map<String, Value>) {
    doc.insert("version".into(), Value::from(5));
    doc.entry("providers")
        .or_insert_with(|| Value::Array(Vec::new()));
}

/// v6 adds usage tracking (prices, budgets, CLI readers) and token management options to
/// `chat`; defaults fill in on load and the existing `chat.smoothReveal` is kept.
fn v5_to_v6(doc: &mut Map<String, Value>) {
    doc.insert("version".into(), Value::from(6));
    doc.entry("usage")
        .or_insert_with(|| Value::Object(Map::new()));
}

/// v7 adds the API agent options; defaults fill in on load.
fn v6_to_v7(doc: &mut Map<String, Value>) {
    doc.insert("version".into(), Value::from(7));
    doc.entry("agent")
        .or_insert_with(|| Value::Object(Map::new()));
}

/// v8 adds workstreams, the attention glow and the Agent Stage (phase 9); defaults fill in
/// on load. Workstreams themselves live in the layout, which the frontend migrates.
fn v7_to_v8(doc: &mut Map<String, Value>) {
    doc.insert("version".into(), Value::from(8));
    doc.entry("workstreams")
        .or_insert_with(|| Value::Object(Map::new()));
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
        assert_eq!(settings.version, CURRENT_VERSION);
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
    fn v2_document_gains_file_settings_and_keeps_its_layout() {
        let raw =
            json!({ "version": 2, "layout": { "version": 1 }, "terminal": { "fontSize": 15 } });
        let settings = migrate(raw).expect("migrate");
        assert_eq!(settings.version, CURRENT_VERSION);
        assert!(!settings.files.show_hidden);
        assert_eq!(settings.terminal.font_size, 15);
        assert!(
            settings.layout.is_some(),
            "the frontend migrates its own layout"
        );
    }

    #[test]
    fn v3_document_gains_character_settings() {
        let raw = json!({ "version": 3, "files": { "showHidden": true } });
        let settings = migrate(raw).expect("migrate");
        assert_eq!(settings.version, CURRENT_VERSION);
        assert!(settings.files.show_hidden);
        assert!(settings.characters.enabled);
        assert_eq!(settings.characters.scale, 3);
        assert!(!settings.characters.on_shells);
    }

    #[test]
    fn v4_document_gains_providers_and_chat_settings() {
        let raw = json!({ "version": 4, "characters": { "scale": 4 }, "layout": { "version": 2 } });
        let settings = migrate(raw).expect("migrate");
        assert_eq!(settings.version, CURRENT_VERSION);
        assert_eq!(settings.characters.scale, 4);
        assert!(settings.providers.is_empty());
        assert!(settings.chat.smooth_reveal);
        assert!(settings.layout.is_some());
    }

    #[test]
    fn v5_providers_round_trip() {
        let raw = json!({
            "version": 5,
            "providers": [{
                "id": "p1", "kind": "openai", "preset": "deepseek", "name": "DS",
                "baseUrl": "https://api.deepseek.com", "defaultModel": "deepseek-chat",
                "maxTokens": 2048, "temperature": 0.5
            }],
            "chat": { "smoothReveal": false }
        });
        let settings = migrate(raw).expect("migrate");
        assert_eq!(settings.providers.len(), 1);
        assert_eq!(settings.providers[0].preset, "deepseek");
        assert_eq!(settings.providers[0].temperature, Some(0.5));
        assert!(!settings.chat.smooth_reveal);
        let again = migrate(serde_json::to_value(&settings).expect("json")).expect("migrate");
        assert_eq!(again, settings);
    }

    #[test]
    fn v5_document_gains_usage_and_token_settings() {
        let raw = json!({
            "version": 5,
            "chat": { "smoothReveal": false },
            "ui": { "activeView": "explorer" },
            "providers": [{ "id": "p1", "kind": "anthropic", "preset": "anthropic" }]
        });
        let settings = migrate(raw).expect("migrate");
        assert_eq!(settings.version, CURRENT_VERSION);
        assert!(
            !settings.chat.smooth_reveal,
            "existing chat options are kept"
        );
        assert!(settings.chat.prompt_caching);
        assert_eq!(settings.chat.default_context_window, 128_000);
        assert_eq!(settings.chat.compact_threshold, 0.7);
        assert!(!settings.chat.auto_compact);
        assert!(settings.usage.track_cli);
        assert!(!settings.usage.cli_subscription);
        assert!(settings.usage.prices.is_empty());
        assert_eq!(settings.usage.budgets.daily, None);
        assert!(!settings.usage.budgets.hard_stop);
        assert_eq!(settings.ui.active_view, "explorer");
        assert_eq!(settings.providers.len(), 1);
    }

    #[test]
    fn v6_usage_round_trips() {
        let raw = json!({
            "version": 6,
            "ui": { "activeView": "usage" },
            "usage": {
                "cliSubscription": true,
                "freeProviders": ["p1"],
                "prices": [{ "model": "x", "input": 1.0, "output": 2.0 }],
                "budgets": { "daily": 5.0, "hardStop": true,
                             "providers": [{ "provider": "codex", "monthly": 30.0 }] }
            }
        });
        let settings = migrate(raw).expect("migrate");
        assert_eq!(settings.ui.active_view, "usage");
        assert!(settings.usage.cli_subscription);
        assert_eq!(settings.usage.prices[0].output, 2.0);
        assert_eq!(settings.usage.budgets.providers[0].monthly, Some(30.0));
        let again = migrate(serde_json::to_value(&settings).expect("json")).expect("migrate");
        assert_eq!(again, settings);
    }

    #[test]
    fn v6_document_gains_agent_settings() {
        let raw = json!({
            "version": 6,
            "usage": { "cliSubscription": true },
            "chat": { "autoCompact": true }
        });
        let settings = migrate(raw).expect("migrate");
        assert_eq!(settings.version, CURRENT_VERSION);
        assert!(settings.usage.cli_subscription, "usage is kept");
        assert!(settings.chat.auto_compact, "chat is kept");
        assert_eq!(settings.agent, crate::settings::AgentSettings::default());
        let again = migrate(serde_json::to_value(&settings).expect("json")).expect("migrate");
        assert_eq!(again, settings);
    }

    #[test]
    fn v7_agent_round_trips() {
        let raw = json!({
            "version": 7,
            "agent": {
                "maxIterations": 40,
                "defaultMode": "readOnly",
                "defaultPrompt": false,
                "showReasoning": false,
                "toolModels": [{ "model": "glm-5.3", "tools": false }]
            }
        });
        let settings = migrate(raw).expect("migrate");
        assert_eq!(settings.agent.max_iterations, 40);
        assert_eq!(
            settings.agent.default_mode,
            crate::conversations::PermissionMode::ReadOnly
        );
        assert!(!settings.agent.default_prompt);
        assert!(!settings.agent.show_reasoning);
        assert!(
            migrate(json!({ "version": 7 }))
                .expect("defaults")
                .agent
                .show_reasoning
        );
        assert_eq!(settings.agent.tools_override("glm-5.3"), Some(false));
    }

    #[test]
    fn wrong_field_type_is_an_error() {
        assert!(migrate(json!({ "version": 1, "language": 42 })).is_err());
    }

    #[test]
    fn v7_document_gains_workstream_settings() {
        let raw = json!({
            "version": 7,
            "agent": { "maxIterations": 40 },
            "appearance": { "agentBold": { "color": "#FF8800", "glow": true } },
            "layout": { "version": 2 }
        });
        let settings = migrate(raw).expect("migrate");
        assert_eq!(settings.version, CURRENT_VERSION);
        assert_eq!(settings.agent.max_iterations, 40, "agent options are kept");
        let ws = &settings.workstreams;
        assert!(ws.glow);
        assert_eq!(ws.glow_color, None, "the glow follows the agent bold color");
        assert!(!ws.notify, "OS notifications are opt-in");
        assert_eq!(ws.default_mode, crate::settings::HandoffMode::SemiAuto);
        assert_eq!(ws.max_rounds, 10);
        assert_eq!(ws.report_cap_kb, 30);
        assert_eq!(ws.stage_scale, 4);
        assert!(ws.stage_animations);
        assert!(settings.layout.is_some());
        let again = migrate(serde_json::to_value(&settings).expect("json")).expect("migrate");
        assert_eq!(again, settings);
    }
}
