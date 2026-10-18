//! Text helpers of the importers: ids, names, one-line descriptions, paths, categories.

const MAX_DESCRIPTION: usize = 500;
const MAX_ID: usize = 48;

/// `a-z0-9-`, at most 48 characters, starting with a letter or digit.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.trim().chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let mut out: String = out.trim_matches('-').chars().take(MAX_ID).collect();
    while out.ends_with('-') {
        out.pop();
    }
    if out.len() < 2 {
        out = format!("item-{out}");
    }
    out
}

/// `code-reviewer` -> `Code Reviewer`.
pub fn humanize(id: &str) -> String {
    id.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().chain(chars).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn cut(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(max - 1).collect();
    out = out.trim_end().to_owned();
    out.push('…');
    out
}

/// The first prose paragraph (no headings, lists, tables, code or HTML), as one line.
pub fn first_paragraph(body: &str) -> Option<String> {
    let mut in_code = false;
    let mut para: Vec<&str> = Vec::new();
    for line in body.lines().chain(std::iter::once("")) {
        let t = line.trim();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_code = !in_code;
            para.clear();
            continue;
        }
        if in_code {
            continue;
        }
        if t.is_empty() {
            if !para.is_empty() {
                return Some(cut(&one_line(&para.join(" ")), MAX_DESCRIPTION));
            }
            continue;
        }
        let special = t.starts_with(['#', '-', '*', '|', '<', '>', '!', '[', '+'])
            || t.chars().next().is_some_and(|c| c.is_ascii_digit());
        if !para.is_empty() || !special {
            para.push(t);
        }
    }
    None
}

/// The folder part of a path (`a/b/c.md` -> `a/b`).
pub fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

/// The file name without its extension.
pub fn stem(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

/// A category from the folder structure: VoltAgent's `categories/01-core-development`,
/// wshobson's `plugins/<plugin>/...`.
pub fn category(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    let after = |name: &str| {
        parts
            .iter()
            .position(|p| *p == name)
            .and_then(|i| parts.get(i + 1))
            .filter(|_| parts.len() > 2)
    };
    let raw = after("categories").or_else(|| after("plugins"))?;
    let trimmed = raw.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-' || c == '_');
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}
