//! Settings schema shared with the frontend (`Settings` in `src/shared/types.ts`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Bump when the on-disk shape changes and add a step in `migrate.rs`.
pub const CURRENT_VERSION: u32 = 3;
pub const SUPPORTED_LANGUAGES: [&str; 5] = ["en", "tr", "de", "zh", "ru"];
pub const DEFAULT_LANGUAGE: &str = "en";
pub const DEFAULT_THEME_ID: &str = "agentflow-dark";
pub const DEFAULT_BOLD_COLOR: &str = "#F5C542";
pub const SIDEBAR_VIEWS: [&str; 3] = ["agents", "explorer", "characters"];
pub const MAX_RECENT_WORKSPACES: usize = 10;
pub const CURSOR_STYLES: [&str; 3] = ["block", "underline", "bar"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    pub language: String,
    pub appearance: Appearance,
    pub ui: UiState,
    /// Folder new terminals start in. `None` means the home directory.
    pub current_workspace: Option<String>,
    /// Most recent first, at most `MAX_RECENT_WORKSPACES`.
    pub recent_workspaces: Vec<String>,
    pub terminal: TerminalSettings,
    pub files: FilesSettings,
    /// Opaque dockview layout owned by the frontend. `None` means "use the default layout".
    pub layout: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TerminalSettings {
    pub font_size: u16,
    pub scrollback: u32,
    /// One of `CURSOR_STYLES`.
    pub cursor_style: String,
    /// Start restored shell panels automatically (agent CLIs never auto-start).
    pub auto_start_shells: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FilesSettings {
    /// Show dotfiles in the file tree and quick open (`.git` always stays hidden).
    pub show_hidden: bool,
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
            current_workspace: None,
            recent_workspaces: Vec::new(),
            terminal: TerminalSettings::default(),
            files: FilesSettings::default(),
            layout: None,
        }
    }
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            font_size: 13,
            scrollback: 5000,
            cursor_style: CURSOR_STYLES[0].into(),
            auto_start_shells: true,
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
