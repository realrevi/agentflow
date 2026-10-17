//! Design detectors: brand colors, fonts, spacing scale and breakpoints from CSS custom
//! properties, `@media` queries, a Tailwind config and Google Fonts links. Results are short
//! suggestions ("primary #1e40af, accent #f59e0b"), never whole files.

use std::collections::BTreeSet;
use std::path::Path;

use super::detect::{re, read_small};

const MAX_COLORS: usize = 12;
const MAX_FONTS: usize = 5;
const MAX_SPACING: usize = 10;
const TAILWIND_BREAKPOINTS: &str = "sm 640px, md 768px, lg 1024px, xl 1280px, 2xl 1536px";

fn texts(root: &Path, files: &[&String]) -> Vec<String> {
    files
        .iter()
        .filter_map(|f| read_small(&root.join(f)))
        .collect()
}

/// Ordered and without repeats.
fn push_unique(list: &mut Vec<String>, value: String, max: usize) {
    if list.len() < max && !list.contains(&value) {
        list.push(value);
    }
}

pub fn colors(root: &Path, styles: &[&String], tailwind: Option<&str>) -> Option<String> {
    let mut found = Vec::new();
    let color = r"(#[0-9a-fA-F]{3,8}\b|(?:rgb|rgba|hsl|hsla|oklch|oklab)\([^)]*\))";
    if let Some(var) = re(&format!(r"--([A-Za-z0-9_-]+)\s*:\s*{color}")) {
        for text in texts(root, styles) {
            for caps in var.captures_iter(&text) {
                let (Some(name), Some(value)) = (caps.get(1), caps.get(2)) else {
                    continue;
                };
                let name = name.as_str().trim_start_matches("color-");
                push_unique(
                    &mut found,
                    format!("{name} {}", value.as_str().to_lowercase()),
                    MAX_COLORS,
                );
            }
        }
    }
    if let (Some(config), Some(pair)) = (
        tailwind,
        re(r#"['"]?([A-Za-z0-9_-]+)['"]?\s*:\s*['"](#[0-9a-fA-F]{3,8})['"]"#),
    ) {
        for caps in pair.captures_iter(config) {
            if let (Some(name), Some(value)) = (caps.get(1), caps.get(2)) {
                let entry = format!("{} {}", name.as_str(), value.as_str().to_lowercase());
                push_unique(&mut found, entry, MAX_COLORS);
            }
        }
    }
    (!found.is_empty()).then(|| found.join(", "))
}

const GENERIC_FONTS: [&str; 14] = [
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
    "ui-sans-serif",
    "ui-serif",
    "ui-monospace",
    "inherit",
    "initial",
    "-apple-system",
    "blinkmacsystemfont",
    "var",
];

/// The first real family of a font stack (`"Inter", system-ui, sans-serif` -> `Inter`).
fn family(stack: &str) -> Option<String> {
    stack
        .split(',')
        .map(|part| part.trim().trim_matches(['"', '\'']).trim())
        .find(|name| {
            !name.is_empty()
                && !name.starts_with("var(")
                && !GENERIC_FONTS.contains(&name.to_ascii_lowercase().as_str())
        })
        .map(str::to_owned)
}

pub fn fonts(
    root: &Path,
    styles: &[&String],
    html: &[&String],
    tailwind: Option<&str>,
) -> Option<String> {
    let mut found = Vec::new();
    let declarations = re(r"(?:font-family|--font[A-Za-z0-9_-]*)\s*:\s*([^;{}]+)");
    for text in texts(root, styles) {
        for caps in declarations.iter().flat_map(|d| d.captures_iter(&text)) {
            if let Some(name) = caps.get(1).and_then(|m| family(m.as_str())) {
                push_unique(&mut found, name, MAX_FONTS);
            }
        }
    }
    if let (Some(config), Some(pattern)) = (tailwind, re(r#"\[\s*['"]([^'"]+)['"]"#)) {
        if let Some(start) = config.find("fontFamily") {
            for caps in pattern.captures_iter(&config[start..]).take(4) {
                if let Some(name) = caps.get(1).and_then(|m| family(m.as_str())) {
                    push_unique(&mut found, name, MAX_FONTS);
                }
            }
        }
    }
    if let Some(google) = re(r"fonts\.googleapis\.com/css2?\?family=([^&:'\x22]+)") {
        for text in texts(root, html) {
            for caps in google.captures_iter(&text) {
                if let Some(name) = caps.get(1) {
                    push_unique(&mut found, name.as_str().replace('+', " "), MAX_FONTS);
                }
            }
        }
    }
    (!found.is_empty()).then(|| found.join(", "))
}

pub fn spacing(root: &Path, styles: &[&String], tailwind: bool) -> Option<String> {
    let mut found = Vec::new();
    if let Some(var) = re(r"--((?:space|spacing|gap|size)[A-Za-z0-9_-]*)\s*:\s*([^;{}]+)") {
        for text in texts(root, styles) {
            for caps in var.captures_iter(&text) {
                if let (Some(name), Some(value)) = (caps.get(1), caps.get(2)) {
                    let entry = format!("{} {}", name.as_str(), value.as_str().trim());
                    push_unique(&mut found, entry, MAX_SPACING);
                }
            }
        }
    }
    if found.is_empty() && tailwind {
        return Some("Tailwind default (0.25rem steps)".into());
    }
    (!found.is_empty()).then(|| found.join(", "))
}

pub fn breakpoints(root: &Path, styles: &[&String], tailwind: bool) -> Option<String> {
    let mut widths = BTreeSet::new();
    if let Some(media) = re(r"@media[^{]*\((?:min|max)-width\s*:\s*(\d+(?:\.\d+)?)(px|em|rem)\)") {
        for text in texts(root, styles) {
            for caps in media.captures_iter(&text) {
                let (Some(value), Some(unit)) = (caps.get(1), caps.get(2)) else {
                    continue;
                };
                let px = value.as_str().parse::<f64>().unwrap_or(0.0)
                    * if unit.as_str() == "px" { 1.0 } else { 16.0 };
                if px > 0.0 {
                    widths.insert(px.round() as u32);
                }
            }
        }
    }
    if widths.is_empty() {
        return tailwind.then(|| TAILWIND_BREAKPOINTS.to_owned());
    }
    let list: Vec<String> = widths.iter().take(8).map(|w| format!("{w}px")).collect();
    Some(list.join(", "))
}
