//! Scalar pieces of the frontmatter reader: quotes, flow lists, comments, escapes.

/// The byte index of the closing quote of a value that starts with `quote`.
pub fn closing(text: &str, quote: char) -> Option<usize> {
    let mut chars = text.char_indices().skip(1).peekable();
    while let Some((i, c)) = chars.next() {
        if quote == '"' && c == '\\' {
            chars.next();
        } else if c == quote {
            if quote == '\'' && chars.peek().is_some_and(|&(_, n)| n == '\'') {
                chars.next();
            } else {
                return Some(i);
            }
        }
    }
    None
}

/// The index of the `]` that closes a flow list starting at 0 (quotes respected).
pub fn flow_end(text: &str) -> Option<usize> {
    let (mut depth, mut quote) = (0usize, None::<char>);
    for (i, c) in text.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '[') => depth += 1,
            (None, ']') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

pub fn split_flow(inner: &str) -> Vec<String> {
    let (mut items, mut current, mut quote) = (Vec::new(), String::new(), None::<char>);
    for c in inner.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => current.push(c),
            (None, '"' | '\'') => quote = Some(c),
            (None, ',') => items.push(std::mem::take(&mut current)),
            (None, c) => current.push(c),
        }
    }
    items.push(current);
    items
        .into_iter()
        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|s| !s.is_empty())
        .collect()
}

/// ` # comment` ends a plain value (a `#` right after text, as in `C#`, does not).
pub fn strip_comment(value: &str) -> &str {
    match value.find(" #") {
        Some(i) => value[..i].trim_end(),
        None => value,
    }
}

pub fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => {}
            Some('0') => {}
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                out.extend(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32));
            }
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}
