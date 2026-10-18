//! The TOML subset of Codex agents and Gemini CLI commands: `key = value` with basic,
//! literal and multi-line strings, string arrays and `[table]` headers (keys come out as
//! `table.key`). Numbers, booleans and inline tables are kept as their raw text.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Toml {
    Str(String),
    List(Vec<String>),
    Raw(String),
}

pub struct Table(Vec<(String, Toml)>);

impl Table {
    pub fn text(&self, key: &str) -> Option<String> {
        match self.0.iter().find(|(k, _)| k == key)?.1.clone() {
            Toml::Str(s) | Toml::Raw(s) => {
                let s = s.trim().to_owned();
                (!s.is_empty()).then_some(s)
            }
            Toml::List(items) => (!items.is_empty()).then(|| items.join(", ")),
        }
    }
}

struct Cursor<'a> {
    s: &'a str,
    i: usize,
}

impl Cursor<'_> {
    fn rest(&self) -> &str {
        &self.s[self.i..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.i += c.len_utf8();
        Some(c)
    }

    fn eat(&mut self, prefix: &str) -> bool {
        if self.rest().starts_with(prefix) {
            self.i += prefix.len();
            true
        } else {
            false
        }
    }

    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.i += 1;
        }
    }

    /// Spaces, line breaks and comments.
    fn skip_blank(&mut self) {
        loop {
            match self.peek() {
                Some(' ' | '\t' | '\r' | '\n') => self.i += 1,
                Some('#') => self.skip_line(),
                _ => return,
            }
        }
    }

    fn skip_line(&mut self) {
        match self.rest().find('\n') {
            Some(n) => self.i += n + 1,
            None => self.i = self.s.len(),
        }
    }

    fn line(&self) -> usize {
        self.s[..self.i].matches('\n').count() + 1
    }

    fn error(&self, what: &str) -> String {
        format!("TOML line {}: {what}", self.line())
    }
}

pub fn parse(text: &str) -> Result<Table, String> {
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let mut c = Cursor { s: &text, i: 0 };
    let mut out = Vec::new();
    let mut table = String::new();
    loop {
        c.skip_blank();
        let Some(first) = c.peek() else { break };
        if first == '[' {
            let line_end = c.rest().find('\n').unwrap_or(c.rest().len());
            let header = c.rest()[..line_end].trim().trim_matches(['[', ']']).trim();
            table = header.to_owned();
            c.skip_line();
            continue;
        }
        let key = key(&mut c)?;
        c.skip_spaces();
        if !c.eat("=") {
            return Err(c.error("expected `=`"));
        }
        c.skip_spaces();
        let value = value(&mut c)?;
        c.skip_spaces();
        if c.peek() == Some('#') {
            c.skip_line();
        } else if !matches!(c.peek(), None | Some('\n')) {
            return Err(c.error("unexpected text after a value"));
        }
        let full = if table.is_empty() {
            key
        } else {
            format!("{table}.{key}")
        };
        out.push((full, value));
    }
    Ok(Table(out))
}

fn key(c: &mut Cursor) -> Result<String, String> {
    match c.peek() {
        Some('"') => basic(c),
        Some('\'') => literal(c),
        _ => {
            let start = c.i;
            while c
                .peek()
                .is_some_and(|ch| ch.is_ascii_alphanumeric() || "_-.".contains(ch))
            {
                c.i += 1;
            }
            if c.i == start {
                return Err(c.error("expected a key"));
            }
            Ok(c.s[start..c.i].to_owned())
        }
    }
}

fn value(c: &mut Cursor) -> Result<Toml, String> {
    if c.rest().starts_with("\"\"\"") {
        return multiline(c, "\"\"\"").map(|s| Toml::Str(unescape(&s)));
    }
    if c.rest().starts_with("'''") {
        return multiline(c, "'''").map(Toml::Str);
    }
    match c.peek() {
        Some('"') => basic(c).map(Toml::Str),
        Some('\'') => literal(c).map(Toml::Str),
        Some('[') => array(c),
        Some('{') => {
            let start = c.i;
            let end = c.rest().find('}').ok_or_else(|| c.error("unclosed {"))?;
            c.i += end + 1;
            Ok(Toml::Raw(c.s[start..c.i].to_owned()))
        }
        _ => {
            let start = c.i;
            while !matches!(c.peek(), None | Some('\n' | '#')) {
                c.i += 1;
            }
            Ok(Toml::Raw(c.s[start..c.i].trim().to_owned()))
        }
    }
}

fn multiline(c: &mut Cursor, quote: &str) -> Result<String, String> {
    c.i += quote.len();
    // A line break right after the opening quotes is not part of the string.
    c.eat("\n");
    let end = c
        .rest()
        .find(quote)
        .ok_or_else(|| c.error("unclosed multi-line string"))?;
    // Up to two quotes right before the closing ones belong to the string.
    let mut len = end;
    while c.rest()[len + quote.len()..].starts_with(&quote[..1]) && len < end + 2 {
        len += 1;
    }
    let raw = c.rest()[..len].to_owned();
    c.i += len + quote.len();
    Ok(raw)
}

fn basic(c: &mut Cursor) -> Result<String, String> {
    c.bump();
    let start = c.i;
    loop {
        match c.bump() {
            Some('\\') => {
                c.bump();
            }
            Some('"') => break,
            Some('\n') | None => return Err(c.error("unclosed string")),
            Some(_) => {}
        }
    }
    let raw = c.s[start..c.i - 1].to_owned();
    Ok(unescape(&raw))
}

fn literal(c: &mut Cursor) -> Result<String, String> {
    c.bump();
    let end = c
        .rest()
        .find(['\'', '\n'])
        .filter(|&n| c.rest()[n..].starts_with('\''))
        .ok_or_else(|| c.error("unclosed string"))?;
    let raw = c.rest()[..end].to_owned();
    c.i += end + 1;
    Ok(raw)
}

fn array(c: &mut Cursor) -> Result<Toml, String> {
    c.bump();
    let mut items = Vec::new();
    loop {
        c.skip_blank();
        match c.peek() {
            Some(']') => {
                c.bump();
                return Ok(Toml::List(items));
            }
            Some(',') => {
                c.bump();
            }
            None => return Err(c.error("unclosed array")),
            _ => match value(c)? {
                Toml::Str(s) | Toml::Raw(s) => items.push(s),
                Toml::List(inner) => items.extend(inner),
            },
        }
    }
}

/// Basic-string escapes; a backslash at a line end joins the next line (TOML's line
/// ending backslash).
fn unescape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('b') => out.push('\u{8}'),
            Some('f') => out.push('\u{c}'),
            Some('e') => out.push('\u{1b}'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(u @ ('u' | 'U')) => {
                let len = if u == 'u' { 4 } else { 8 };
                let hex: String = chars.by_ref().take(len).collect();
                out.extend(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32));
            }
            Some(' ' | '\t' | '\n') => {
                while chars.peek().is_some_and(|c| c.is_whitespace()) {
                    chars.next();
                }
            }
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}
