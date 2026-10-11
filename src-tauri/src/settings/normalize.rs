//! Repairs out-of-range or unknown values so the rest of the app can trust settings.

use super::model::{
    Settings, CURRENT_VERSION, DEFAULT_BOLD_COLOR, DEFAULT_LANGUAGE, DEFAULT_THEME_ID,
    SIDEBAR_VIEWS, SUPPORTED_LANGUAGES,
};
use crate::util::is_hex_color;

const SIDEBAR_MIN_WIDTH: u32 = 180;
const SIDEBAR_MAX_WIDTH: u32 = 600;
const MAX_THEME_ID_LEN: usize = 128;

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
    use crate::settings::{AgentBold, Appearance, UiState};
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
}
