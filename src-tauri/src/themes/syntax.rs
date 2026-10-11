//! Editor syntax colors from a VS Code theme's `tokenColors` (TextMate scopes). They are
//! merged into `colors` as `agentflow.syntax.<category>`; the frontend maps them to
//! `--af-syntax-*` and falls back to the terminal ANSI palette for missing ones.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::util::is_hex_color;

pub const SYNTAX_KEY_PREFIX: &str = "agentflow.syntax.";

/// Each category tries its scopes in order (mirrors `src/features/theme/syntax.ts`).
const CATEGORIES: &[(&str, &[&str])] = &[
    ("keyword", &["keyword", "storage.type", "storage.modifier"]),
    ("string", &["string"]),
    ("number", &["constant.numeric"]),
    (
        "constant",
        &["constant.language", "variable.other.constant", "constant"],
    ),
    ("comment", &["comment"]),
    ("function", &["entity.name.function", "support.function"]),
    (
        "type",
        &[
            "entity.name.type",
            "entity.name.class",
            "support.type",
            "support.class",
        ],
    ),
    (
        "property",
        &[
            "variable.other.property",
            "support.type.property-name",
            "meta.object-literal.key",
        ],
    ),
    ("tag", &["entity.name.tag"]),
    ("attribute", &["entity.other.attribute-name"]),
    ("variable", &["variable.other.readwrite", "variable"]),
    (
        "meta",
        &[
            "meta.decorator",
            "meta.preprocessor",
            "keyword.control.directive",
        ],
    ),
    ("heading", &["markup.heading", "entity.name.section"]),
];

struct Rule<'a> {
    selector: &'a str,
    color: &'a str,
}

/// `agentflow.syntax.*` colors for the categories the theme defines.
pub fn syntax_colors(token_colors: Option<&Value>) -> BTreeMap<String, String> {
    let rules = token_colors.map(rules).unwrap_or_default();
    CATEGORIES
        .iter()
        .filter_map(|(category, scopes)| {
            let color = scopes.iter().find_map(|scope| best_match(&rules, scope))?;
            Some((format!("{SYNTAX_KEY_PREFIX}{category}"), color.to_owned()))
        })
        .collect()
}

/// Simple selectors with a valid foreground (descendant selectors like `meta.tag string`
/// are skipped).
fn rules(token_colors: &Value) -> Vec<Rule<'_>> {
    let Some(entries) = token_colors.as_array() else {
        return Vec::new();
    };
    let mut rules = Vec::new();
    for entry in entries {
        let color = entry
            .pointer("/settings/foreground")
            .and_then(Value::as_str);
        let Some(color) = color.filter(|color| is_hex_color(color)) else {
            continue;
        };
        let scopes: Vec<&str> = match entry.get("scope") {
            Some(Value::String(list)) => list.split(',').collect(),
            Some(Value::Array(list)) => list.iter().filter_map(Value::as_str).collect(),
            _ => continue,
        };
        for selector in scopes.into_iter().map(str::trim) {
            if !selector.is_empty() && !selector.contains(' ') {
                rules.push(Rule { selector, color });
            }
        }
    }
    rules
}

/// The most specific rule whose selector is a prefix of `scope` (later rules win ties).
fn best_match<'a>(rules: &[Rule<'a>], scope: &str) -> Option<&'a str> {
    rules
        .iter()
        .filter(|rule| {
            scope == rule.selector
                || scope
                    .strip_prefix(rule.selector)
                    .is_some_and(|rest| rest.starts_with('.'))
        })
        .max_by_key(|rule| rule.selector.len())
        .map(|rule| rule.color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn picks_the_most_specific_simple_rule() {
        let tokens = json!([
            { "settings": { "foreground": "#ffffff" } },
            { "scope": "comment", "settings": { "foreground": "#6a9955", "fontStyle": "italic" } },
            { "scope": ["keyword", "storage.type"], "settings": { "foreground": "#569cd6" } },
            { "scope": "keyword.control", "settings": { "foreground": "#c586c0" } },
            { "scope": "string, string.quoted", "settings": { "foreground": "#ce9178" } },
            { "scope": "meta.tag string", "settings": { "foreground": "#000000" } },
            { "scope": "entity.name.function", "settings": { "foreground": "orange" } },
            { "scope": "constant", "settings": { "foreground": "#4fc1ff" } }
        ]);
        let colors = syntax_colors(Some(&tokens));
        assert_eq!(colors["agentflow.syntax.comment"], "#6a9955");
        assert_eq!(colors["agentflow.syntax.keyword"], "#569cd6");
        assert_eq!(colors["agentflow.syntax.string"], "#ce9178");
        // `constant` is a prefix of `constant.numeric` and `constant.language`.
        assert_eq!(colors["agentflow.syntax.number"], "#4fc1ff");
        assert_eq!(colors["agentflow.syntax.constant"], "#4fc1ff");
        assert!(
            !colors.contains_key("agentflow.syntax.function"),
            "invalid color"
        );
        assert!(!colors.contains_key("agentflow.syntax.tag"));
    }

    #[test]
    fn missing_or_odd_token_colors_give_nothing() {
        assert!(syntax_colors(None).is_empty());
        assert!(syntax_colors(Some(&json!("./tokens.json"))).is_empty());
        assert!(syntax_colors(Some(&json!([{ "scope": 3 }]))).is_empty());
    }
}
