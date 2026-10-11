//! Settings schema shared with the frontend (`Settings` in `src/shared/types.ts`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Bump when the on-disk shape changes and add a step in `migrate.rs`.
pub const CURRENT_VERSION: u32 = 1;
pub const SUPPORTED_LANGUAGES: [&str; 5] = ["en", "tr", "de", "zh", "ru"];
pub const DEFAULT_LANGUAGE: &str = "en";
pub const DEFAULT_THEME_ID: &str = "agentflow-dark";
pub const DEFAULT_BOLD_COLOR: &str = "#F5C542";
pub const SIDEBAR_VIEWS: [&str; 3] = ["agents", "explorer", "characters"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    pub language: String,
    pub appearance: Appearance,
    pub ui: UiState,
    /// Opaque dockview layout owned by the frontend. `None` means "use the default layout".
    pub layout: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Appearance {
    pub theme_id: String,
    /// Overrides the theme's base text color when set.
    pub text_color: Option<String>,
    /// Overrides the theme's accent color when set.
    pub accent_color: Option<String>,
    pub agent_bold: AgentBold,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AgentBold {
    pub color: String,
    pub glow: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UiState {
    pub sidebar_visible: bool,
    pub sidebar_width: u32,
    pub active_view: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            language: DEFAULT_LANGUAGE.into(),
            appearance: Appearance::default(),
            ui: UiState::default(),
            layout: None,
        }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme_id: DEFAULT_THEME_ID.into(),
            text_color: None,
            accent_color: None,
            agent_bold: AgentBold::default(),
        }
    }
}

impl Default for AgentBold {
    fn default() -> Self {
        Self {
            color: DEFAULT_BOLD_COLOR.into(),
            glow: true,
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            sidebar_visible: true,
            sidebar_width: 260,
            active_view: SIDEBAR_VIEWS[0].into(),
        }
    }
}
