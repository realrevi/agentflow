//! Validation of VS Code color theme documents.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use super::jsonc::strip_jsonc;
use super::model::ThemeKind;
use crate::error::{AppError, AppResult};
use crate::util::is_hex_color;

const MAX_NAME_CHARS: usize = 80;
const FALLBACK_NAME: &str = "Imported theme";

#[derive(Debug, Clone)]
pub struct ParsedTheme {
    pub name: String,
    pub kind: ThemeKind,
    /// Only entries with valid hex values; everything else is dropped.
    pub colors: BTreeMap<String, String>,
    /// The full document (including `tokenColors`), normalized to plain JSON.
    pub document: Map<String, Value>,
}

/// Parses and validates a theme. `fallback_name` is used when the file has no `name`.
pub fn parse_theme(source: &str, fallback_name: &str) -> AppResult<ParsedTheme> {
    let value: Value = serde_json::from_str(&strip_jsonc(source))
        .map_err(|err| invalid(format!("not valid JSON ({err})")))?;
    let Value::Object(mut document) = value else {
        return Err(invalid("expected a JSON object at the top level"));
    };

    let colors = match document.get("colors") {
        Some(Value::Object(raw)) => valid_colors(raw),
        Some(_) => return Err(invalid("\"colors\" must be an object")),
        None if document.contains_key("include") => {
            return Err(invalid("themes that use \"include\" are not supported yet"))
        }
        None => return Err(invalid("missing \"colors\" object")),
    };
    if colors.is_empty() {
        return Err(invalid("\"colors\" has no valid hex color values"));
    }
    if let Some(tokens) = document.get("tokenColors") {
        if !(tokens.is_array() || tokens.is_string()) {
            return Err(invalid("\"tokenColors\" must be an array or a path string"));
        }
    }

    let name = theme_name(&document, fallback_name);
    let kind = theme_kind(&document, &colors);
    document.insert("name".into(), Value::String(name.clone()));
    Ok(ParsedTheme {
        name,
        kind,
        colors,
        document,
    })
}

fn invalid(reason: impl Into<String>) -> AppError {
    AppError::InvalidTheme(reason.into())
}

fn valid_colors(raw: &Map<String, Value>) -> BTreeMap<String, String> {
    raw.iter()
        .filter_map(|(key, value)| match value {
            Value::String(color) if !key.is_empty() && is_hex_color(color) => {
                Some((key.clone(), color.clone()))
            }
            _ => None,
        })
        .collect()
}

fn theme_name(document: &Map<String, Value>, fallback: &str) -> String {
    let from_doc = document.get("name").and_then(Value::as_str).map(str::trim);
    let name = match from_doc {
        Some(name) if !name.is_empty() => name,
        _ if !fallback.trim().is_empty() => fallback.trim(),
        _ => FALLBACK_NAME,
    };
    name.chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME_CHARS)
        .collect()
}

fn theme_kind(document: &Map<String, Value>, colors: &BTreeMap<String, String>) -> ThemeKind {
    match document.get("type").and_then(Value::as_str) {
        Some("light" | "hcLight" | "vs") => ThemeKind::Light,
        Some("dark" | "hc" | "hcDark" | "vs-dark" | "hc-black") => ThemeKind::Dark,
        _ => colors
            .get("editor.background")
            .and_then(|bg| relative_luminance(bg))
            .map_or(ThemeKind::Dark, |lum| {
                if lum > 0.5 {
                    ThemeKind::Light
                } else {
                    ThemeKind::Dark
                }
            }),
    }
}

/// Approximate luminance (0..1) of a validated hex color, ignoring alpha.
fn relative_luminance(hex: &str) -> Option<f64> {
    let digits = hex.strip_prefix('#')?;
    let (r, g, b) = match digits.len() {
        3 | 4 => {
            let d = |i: usize| u8::from_str_radix(&digits[i..=i].repeat(2), 16).ok();
            (d(0)?, d(1)?, d(2)?)
        }
        6 | 8 => {
            let d = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
            (d(0)?, d(2)?, d(4)?)
        }
        _ => return None,
    };
    Some((0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b)) / 255.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind_of(source: &str) -> ThemeKind {
        parse_theme(source, "x").expect("valid").kind
    }

    #[test]
    fn accepts_jsonc_and_keeps_only_valid_colors() {
        let source = r##"{
            // comment
            "name": "  Night Owl  ",
            "type": "dark",
            "colors": {
                "editor.background": "#011627",
                "editor.foreground": "#d6deebcc",
                "sideBar.background": "rgb(1,2,3)",
                "tab.border": 42,
            },
            "tokenColors": [{ "scope": "comment", "settings": { "foreground": "#637777" } }],
        }"##;
        let theme = parse_theme(source, "fallback").expect("valid");
        assert_eq!(theme.name, "Night Owl");
        assert_eq!(theme.kind, ThemeKind::Dark);
        assert_eq!(theme.colors.len(), 2);
        assert_eq!(theme.colors["editor.foreground"], "#d6deebcc");
        assert!(theme.document["tokenColors"].is_array());
        assert_eq!(theme.document["name"], "Night Owl");
    }

    #[test]
    fn rejects_invalid_documents() {
        let cases = [
            ("not json", "not valid JSON"),
            ("[1, 2]", "top level"),
            (r#"{ "name": "x" }"#, "missing \"colors\""),
            (r#"{ "include": "./base.json" }"#, "include"),
            (r#"{ "colors": [] }"#, "must be an object"),
            (r#"{ "colors": { "a": "red" } }"#, "no valid hex"),
            (
                r##"{ "colors": { "a": "#fff" }, "tokenColors": 3 }"##,
                "tokenColors",
            ),
        ];
        for (source, expected) in cases {
            match parse_theme(source, "x") {
                Err(AppError::InvalidTheme(reason)) => {
                    assert!(reason.contains(expected), "{source}: got {reason}")
                }
                other => panic!("{source}: expected InvalidTheme, got {other:?}"),
            }
        }
    }

    #[test]
    fn falls_back_to_file_name() {
        let theme = parse_theme(r##"{ "colors": { "a": "#fff" } }"##, "my-theme").expect("valid");
        assert_eq!(theme.name, "my-theme");
        let theme = parse_theme(r##"{ "colors": { "a": "#fff" } }"##, " ").expect("valid");
        assert_eq!(theme.name, FALLBACK_NAME);
    }

    #[test]
    fn detects_kind_from_type_or_background() {
        assert_eq!(
            kind_of(r##"{ "type": "hcLight", "colors": { "a": "#000" } }"##),
            ThemeKind::Light
        );
        assert_eq!(
            kind_of(r##"{ "colors": { "editor.background": "#fafafa" } }"##),
            ThemeKind::Light
        );
        assert_eq!(
            kind_of(r##"{ "colors": { "editor.background": "#1e1e1e" } }"##),
            ThemeKind::Dark
        );
        assert_eq!(
            kind_of(r##"{ "colors": { "a": "#fff" } }"##),
            ThemeKind::Dark
        );
    }
}
