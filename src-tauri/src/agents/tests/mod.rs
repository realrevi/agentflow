//! Installer tests: template format, placeholders, managed sections, renderers (snapshots),
//! install/update/uninstall with the registry, path safety, detectors and profiles.

mod detect;
mod install;
mod render;
mod section;
mod skills;
mod template;

use serde_json::json;

use super::template::Template;

/// A small valid template with one question of each common kind.
pub(super) fn sample() -> Template {
    serde_json::from_value(json!({
        "schema": 1,
        "id": "demo-agent",
        "name": "Demo Agent",
        "description": "Checks demo things.",
        "version": "1.0.0",
        "instructions": "You help {{workspaceName}}.\nColors: {{colors}}\nStrict: {{strict}}\nFocus: {{focus}}",
        "questions": [
            { "id": "colors", "label": "Colors", "type": "color", "detect": "colors" },
            { "id": "strict", "label": "Strict?", "type": "boolean", "default": true },
            { "id": "focus", "label": "Focus", "type": "multiChoice", "options": ["a", "b"], "default": ["a"] }
        ]
    }))
    .expect("sample template")
}
