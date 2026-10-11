//! Repairs out-of-range or unknown values so the rest of the app can trust settings.

use super::model::{
    Settings, TerminalSettings, CURRENT_VERSION, CURSOR_STYLES, DEFAULT_BOLD_COLOR,
    DEFAULT_LANGUAGE, DEFAULT_THEME_ID, MAX_RECENT_WORKSPACES, SIDEBAR_VIEWS, SUPPORTED_LANGUAGES,
};
use crate::util::is_hex_color;

const SIDEBAR_MIN_WIDTH: u32 = 180;
const SIDEBAR_MAX_WIDTH: u32 = 600;
const MAX_THEME_ID_LEN: usize = 128;
const FONT_SIZE_RANGE: (u16, u16) = (8, 32);
const MAX_SCROLLBACK: u32 = 100_000;

impl Settings {
    pub fn normalized(mut self) -> Self {
        self.version = CURRENT_VERSION;
        if !SUPPORTED_LANGUAGES.contains(&self.language.as_str()) {
            self.language = DEFAULT_LANGUAGE.into();
        }

        let appearance = &mut self.appearance;
        if !is_valid_theme_id(&appearance.theme_id) {
            appearance.theme_id = DEFAULT_THEME_ID.into();
        }
        appearance.text_color = appearance.text_color.take().filter(|c| is_hex_color(c));
        appearance.accent_color = appearance.accent_color.take().filter(|c| is_hex_color(c));
        if !is_hex_color(&appearance.agent_bold.color) {
            appearance.agent_bold.color = DEFAULT_BOLD_COLOR.into();
        }

        let ui = &mut self.ui;
        ui.sidebar_width = ui.sidebar_width.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
        if !SIDEBAR_VIEWS.contains(&ui.active_view.as_str()) {
            ui.active_view = SIDEBAR_VIEWS[0].into();
        }

        // Paths are kept verbatim: leading/trailing spaces are legal in folder names.
        self.current_workspace = self
            .current_workspace
            .take()
            .filter(|path| !path.trim().is_empty());
        self.recent_workspaces = normalize_recent(std::mem::take(&mut self.recent_workspaces));
        normalize_terminal(&mut self.terminal);

        if !self
            .layout
            .as_ref()
            .is_some_and(|layout| layout.is_object())
        {
            self.layout = None;
        }
        self
    }
}

/// Not blank, without duplicates, most recent first, at most 10.
fn normalize_recent(paths: Vec<String>) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for path in paths {
        if !path.trim().is_empty() && !seen.contains(&path) {
            seen.push(path);
        }
    }
    seen.truncate(MAX_RECENT_WORKSPACES);
    seen
}

fn normalize_terminal(terminal: &mut TerminalSettings) {
    let (min, max) = FONT_SIZE_RANGE;
    terminal.font_size = terminal.font_size.clamp(min, max);
    terminal.scrollback = terminal.scrollback.min(MAX_SCROLLBACK);
    if !CURSOR_STYLES.contains(&terminal.cursor_style.as_str()) {
        terminal.cursor_style = CURSOR_STYLES[0].into();
    }
}

fn is_valid_theme_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_THEME_ID_LEN
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{AgentBold, Appearance, FilesSettings, UiState};
    use serde_json::json;

    #[test]
    fn repairs_invalid_values() {
        let settings = Settings {
            version: 0,
            language: "fr".into(),
            appearance: Appearance {
                theme_id: "../../etc/passwd".into(),
                text_color: Some("red".into()),
                accent_color: Some("#12345".into()),
                agent_bold: AgentBold {
                    color: "gold".into(),
                    glow: true,
                },
            },
            ui: UiState {
                sidebar_visible: true,
                sidebar_width: 5,
                active_view: "nope".into(),
            },
            current_workspace: None,
            recent_workspaces: Vec::new(),
            terminal: TerminalSettings::default(),
            files: FilesSettings::default(),
            layout: Some(json!([1, 2, 3])),
        };

        let fixed = settings.normalized();
        let expected_ui = UiState {
            sidebar_width: SIDEBAR_MIN_WIDTH,
            ..UiState::default()
        };
        assert_eq!(
            fixed,
            Settings {
                ui: expected_ui,
                ..Settings::default()
            }
        );
    }

    #[test]
    fn keeps_valid_values() {
        let settings = Settings {
            language: "zh".into(),
            appearance: Appearance {
                theme_id: "user:my-theme".into(),
                text_color: Some("#eeeeee".into()),
                ..Appearance::default()
            },
            layout: Some(json!({ "grid": {} })),
            ..Settings::default()
        };

        assert_eq!(settings.clone().normalized(), settings);
    }

    #[test]
    fn cleans_workspaces_and_terminal_settings() {
        let mut recent: Vec<String> = (0..15).map(|i| format!("/work/{i}")).collect();
        recent.insert(1, "/work/0".into());
        recent.insert(2, "   ".into());
        let settings = Settings {
            current_workspace: Some("  ".into()),
            recent_workspaces: recent,
            terminal: TerminalSettings {
                font_size: 99,
                scrollback: 5_000_000,
                cursor_style: "blink".into(),
                auto_start_shells: false,
            },
            ..Settings::default()
        };
        let fixed = settings.normalized();
        assert_eq!(fixed.current_workspace, None);
        assert_eq!(fixed.recent_workspaces.len(), MAX_RECENT_WORKSPACES);
        assert_eq!(fixed.recent_workspaces[0], "/work/0");
        assert_eq!(fixed.recent_workspaces[1], "/work/1");
        let spaced = Settings {
            current_workspace: Some(" my folder ".into()),
            ..Settings::default()
        };
        assert_eq!(
            spaced.normalized().current_workspace.as_deref(),
            Some(" my folder ")
        );
        assert_eq!(fixed.terminal.font_size, 32);
        assert_eq!(fixed.terminal.scrollback, MAX_SCROLLBACK);
        assert_eq!(fixed.terminal.cursor_style, "block");
        assert!(!fixed.terminal.auto_start_shells);
    }
}
