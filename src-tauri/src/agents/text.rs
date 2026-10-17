//! Safe scalars for the generated YAML frontmatter and TOML: values are quoted and escaped
//! whenever they could be read as anything but a plain string.

/// A YAML scalar: plain when that is unambiguous, else a double-quoted string.
pub fn yaml_scalar(value: &str) -> String {
    let value = value.trim();
    let plain = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || " .,()/_-+'@".contains(c))
        && !value.starts_with(['-', '@', '\'', ' '])
        && !matches!(
            value.to_ascii_lowercase().as_str(),
            "true" | "false" | "yes" | "no" | "null" | "on" | "off" | "~"
        )
        && value.parse::<f64>().is_err();
    if plain {
        value.to_owned()
    } else {
        quoted(value)
    }
}

fn quoted(value: &str) -> String {
    let mut out = String::from('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A YAML flow list: `[read_file, search]`.
pub fn yaml_list(items: &[String]) -> String {
    let items: Vec<String> = items.iter().map(|item| yaml_scalar(item)).collect();
    format!("[{}]", items.join(", "))
}

/// A TOML basic string (same escapes as YAML double quotes).
pub fn toml_string(value: &str) -> String {
    quoted(value)
}

/// A TOML multi-line basic string; backslashes and runs of quotes are escaped.
pub fn toml_multiline(value: &str) -> String {
    let mut out = String::from("\"\"\"\n");
    let mut quotes = 0;
    for c in value.chars() {
        match c {
            '\\' => {
                out.push_str("\\\\");
                quotes = 0;
            }
            '"' => {
                quotes += 1;
                if quotes == 3 {
                    out.push_str("\\\"");
                    quotes = 0;
                } else {
                    out.push('"');
                }
            }
            '\r' => {}
            c if c.is_control() && c != '\n' && c != '\t' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
                quotes = 0;
            }
            c => {
                out.push(c);
                quotes = 0;
            }
        }
    }
    // A line break before the closing quotes keeps a trailing quote from touching them.
    out.push_str("\n\"\"\"");
    out
}
