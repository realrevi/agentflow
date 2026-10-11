//! VS Code theme files are often JSONC. This strips comments, trailing commas and a BOM
//! so `serde_json` can parse them, without touching anything inside strings.

pub fn strip_jsonc(source: &str) -> String {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    remove_trailing_commas(&remove_comments(source))
}

fn remove_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match (c, chars.peek()) {
            ('"', _) => {
                in_string = true;
                out.push(c);
            }
            ('/', Some('/')) => {
                // Line comment: drop until (not including) the newline.
                while chars.peek().is_some_and(|&next| next != '\n') {
                    chars.next();
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut prev = '\0';
                for next in chars.by_ref() {
                    if prev == '*' && next == '/' {
                        break;
                    }
                    prev = next;
                }
                out.push(' ');
            }
            _ => out.push(c),
        }
    }
    out
}

fn remove_trailing_commas(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut in_string = false;
    let mut escaped = false;

    for (i, &c) in chars.iter().enumerate() {
        if in_string {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
        } else if c == '"' {
            in_string = true;
        } else if c == ',' {
            let next = chars[i + 1..].iter().find(|ch| !ch.is_whitespace());
            if matches!(next, Some('}' | ']')) {
                continue;
            }
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::strip_jsonc;
    use serde_json::{json, Value};

    fn parse(source: &str) -> Value {
        serde_json::from_str(&strip_jsonc(source)).expect("valid after stripping")
    }

    #[test]
    fn strips_comments_and_trailing_commas() {
        let source = "\u{feff}{\n  // line\n  \"a\": 1, /* block */\n  \"b\": [1, 2,],\n}";
        assert_eq!(parse(source), json!({ "a": 1, "b": [1, 2] }));
    }

    #[test]
    fn keeps_comment_like_text_inside_strings() {
        let source = r#"{ "url": "https://x.dev/*not*/", "q": "say \"//hi\",", }"#;
        assert_eq!(
            parse(source),
            json!({ "url": "https://x.dev/*not*/", "q": "say \"//hi\"," })
        );
    }

    #[test]
    fn plain_json_is_unchanged() {
        let source = r#"{"a":{"b":[true,null,"c"]}}"#;
        assert_eq!(strip_jsonc(source), source);
    }
}
