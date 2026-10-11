//! Built-in themes, embedded from the repo's `themes/` folder at compile time.

pub struct BuiltinTheme {
    pub id: &'static str,
    pub source: &'static str,
}

pub const BUILTIN_THEMES: [BuiltinTheme; 3] = [
    BuiltinTheme {
        id: "agentflow-dark",
        source: include_str!("../../../themes/agentflow-dark.json"),
    },
    BuiltinTheme {
        id: "agentflow-light",
        source: include_str!("../../../themes/agentflow-light.json"),
    },
    BuiltinTheme {
        id: "retro-crt",
        source: include_str!("../../../themes/retro-crt.json"),
    },
];

pub fn find(id: &str) -> Option<&'static BuiltinTheme> {
    BUILTIN_THEMES.iter().find(|theme| theme.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::{parse_theme, ThemeKind};

    #[test]
    fn all_builtin_themes_are_valid() {
        let kinds: Vec<_> = BUILTIN_THEMES
            .iter()
            .map(|theme| parse_theme(theme.source, theme.id).expect(theme.id).kind)
            .collect();
        assert_eq!(kinds, [ThemeKind::Dark, ThemeKind::Light, ThemeKind::Dark]);
    }

    #[test]
    fn builtin_themes_cover_the_core_ui_keys() {
        let required = [
            "editor.background",
            "editor.foreground",
            "sideBar.background",
            "activityBar.background",
            "statusBar.background",
            "tab.activeBackground",
            "panel.background",
            "terminal.ansiBrightWhite",
        ];
        for theme in &BUILTIN_THEMES {
            let parsed = parse_theme(theme.source, theme.id).expect(theme.id);
            for key in required {
                assert!(parsed.colors.contains_key(key), "{} lacks {key}", theme.id);
            }
        }
    }
}
