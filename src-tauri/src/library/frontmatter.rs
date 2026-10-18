//! YAML frontmatter (`---` ... `---` at the top of Markdown) as library files use it: a
//! small, forgiving subset of YAML. Scalars (plain, quoted, `|` / `>` blocks), lists (`- a`
//! or `[a, b]`) and nested maps; anything fancier (anchors, tags) stays plain text. Plain
//! values are taken as written, so Cursor's unquoted `globs: **/*.ts` works.

use super::scalars::{closing, flow_end, split_flow, strip_comment, unescape};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Yaml {
    Str(String),
    List(Vec<String>),
    Map(Vec<(String, Yaml)>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Doc {
    pub fields: Vec<(String, Yaml)>,
    /// The Markdown after the frontmatter (the whole text without one).
    pub body: String,
    pub has_frontmatter: bool,
}

impl Doc {
    pub fn get(&self, key: &str) -> Option<&Yaml> {
        get(&self.fields, key)
    }

    /// A non-empty scalar (a list is joined with `, `).
    pub fn text(&self, key: &str) -> Option<String> {
        self.get(key).and_then(text)
    }

    /// A list, or a comma-separated scalar.
    pub fn list(&self, key: &str) -> Vec<String> {
        self.get(key).map(list).unwrap_or_default()
    }

    /// A scalar inside a nested map (`metadata.author`).
    pub fn nested(&self, map: &str, key: &str) -> Option<String> {
        match self.get(map) {
            Some(Yaml::Map(fields)) => get(fields, key).and_then(text),
            _ => None,
        }
    }
}

fn get<'a>(fields: &'a [(String, Yaml)], key: &str) -> Option<&'a Yaml> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn text(value: &Yaml) -> Option<String> {
    let text = match value {
        Yaml::Str(s) => s.trim().to_owned(),
        Yaml::List(items) => items.join(", "),
        Yaml::Map(_) => return None,
    };
    (!text.is_empty()).then_some(text)
}

fn list(value: &Yaml) -> Vec<String> {
    match value {
        Yaml::Str(s) => s
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
        Yaml::List(items) => items.clone(),
        Yaml::Map(_) => Vec::new(),
    }
}

/// Splits and parses `text`. Line endings become `\n`; a missing frontmatter is not an
/// error (the whole text is the body), a broken one is.
pub fn parse(text: &str) -> Result<Doc, String> {
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let Some(rest) = text.strip_prefix("---\n") else {
        return Ok(Doc {
            body: text,
            ..Doc::default()
        });
    };
    let mut yaml_end = None;
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        let bare = line.trim_end();
        if bare == "---" || bare == "..." {
            yaml_end = Some((offset, offset + line.len()));
            break;
        }
        offset += line.len();
    }
    let (end, body_start) = yaml_end.ok_or("the frontmatter is not closed with ---")?;
    let fields = Parser::new(&rest[..end]).map(0)?;
    Ok(Doc {
        fields,
        body: rest[body_start..].trim_start_matches('\n').to_owned(),
        has_frontmatter: true,
    })
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}

fn is_content(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && !trimmed.starts_with('#')
}

/// `key: rest` (the key may be quoted); `None` for list items and plain text.
fn split_key(content: &str) -> Option<(String, &str)> {
    if content.starts_with("- ") || content == "-" {
        return None;
    }
    let (key, after) = match content.chars().next()? {
        quote @ ('"' | '\'') => {
            let close = content[1..].find(quote)? + 1;
            (content[1..close].to_owned(), &content[close + 1..])
        }
        _ => {
            let colon = content
                .char_indices()
                .find(|&(i, c)| {
                    c == ':'
                        && content[i + 1..]
                            .chars()
                            .next()
                            .is_none_or(|n| n == ' ' || n == '\t')
                })?
                .0;
            (content[..colon].trim().to_owned(), &content[colon..])
        }
    };
    let rest = after.strip_prefix(':')?;
    let valid = !key.is_empty() && !key.contains(['[', ']', '{', '}', ',', '#']);
    valid.then_some((key, rest))
}

impl<'a> Parser<'a> {
    fn new(yaml: &'a str) -> Self {
        Self {
            lines: yaml.lines().collect(),
            pos: 0,
        }
    }

    /// The next line that holds something (blank and comment lines are skipped).
    fn peek(&mut self) -> Option<&'a str> {
        while let Some(line) = self.lines.get(self.pos) {
            if is_content(line) {
                return Some(line);
            }
            self.pos += 1;
        }
        None
    }

    fn map(&mut self, at: usize) -> Result<Vec<(String, Yaml)>, String> {
        let mut fields = Vec::new();
        while let Some(line) = self.peek() {
            let ind = indent(line);
            if ind < at {
                break;
            }
            let line_no = self.pos + 1;
            if ind > at {
                return Err(format!(
                    "frontmatter line {line_no}: unexpected indentation"
                ));
            }
            let (key, rest) = split_key(&line[ind..])
                .ok_or_else(|| format!("frontmatter line {line_no}: expected `key: value`"))?;
            self.pos += 1;
            let value = self.value(rest.trim(), ind)?;
            fields.push((key, value));
        }
        Ok(fields)
    }

    fn value(&mut self, rest: &str, at: usize) -> Result<Yaml, String> {
        if rest.is_empty() || rest.starts_with('#') {
            let Some(next) = self.peek() else {
                return Ok(Yaml::Str(String::new()));
            };
            let child = indent(next);
            let content = &next[child..];
            let item = content.starts_with("- ") || content == "-";
            return if child > at && item || child == at && item {
                Ok(Yaml::List(self.list(child)?))
            } else if child > at && split_key(content).is_some() {
                Ok(Yaml::Map(self.map(child)?))
            } else if child > at {
                Ok(Yaml::Str(self.plain(String::new(), at)))
            } else {
                Ok(Yaml::Str(String::new()))
            };
        }
        Ok(match rest.as_bytes()[0] {
            b'|' | b'>' => Yaml::Str(self.block(rest, at)),
            b'[' => Yaml::List(self.flow(rest)?),
            b'"' | b'\'' => Yaml::Str(self.quoted(rest)?),
            _ => Yaml::Str(self.plain(strip_comment(rest).to_owned(), at)),
        })
    }

    /// A plain scalar and its more-indented continuation lines, folded with spaces.
    fn plain(&mut self, mut text: String, at: usize) -> String {
        while let Some(line) = self.lines.get(self.pos) {
            if is_content(line) && indent(line) <= at {
                break;
            }
            if line.trim().is_empty() {
                text.push('\n');
            } else {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push(' ');
                }
                text.push_str(strip_comment(line.trim()));
            }
            self.pos += 1;
        }
        text.trim().to_owned()
    }

    /// `|` keeps line breaks, `>` folds them; the block is every more-indented line.
    fn block(&mut self, header: &str, at: usize) -> String {
        let folded = header.starts_with('>');
        let mut lines = Vec::new();
        while let Some(line) = self.lines.get(self.pos) {
            if !line.trim().is_empty() && indent(line) <= at {
                break;
            }
            lines.push(*line);
            self.pos += 1;
        }
        let base = lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| indent(l))
            .min()
            .unwrap_or(0);
        let lines: Vec<&str> = lines.iter().map(|l| l.get(base..).unwrap_or("")).collect();
        let text = if folded {
            let mut out = String::new();
            for line in lines {
                if line.is_empty() {
                    out.push('\n');
                } else {
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push(' ');
                    }
                    out.push_str(line);
                }
            }
            out
        } else {
            lines.join("\n")
        };
        text.trim_end().to_owned()
    }

    fn list(&mut self, at: usize) -> Result<Vec<String>, String> {
        let mut items = Vec::new();
        while let Some(line) = self.peek() {
            let content = &line[indent(line)..];
            if indent(line) != at || !(content.starts_with("- ") || content == "-") {
                break;
            }
            self.pos += 1;
            let item = content[1..].trim();
            let value = match item.as_bytes().first() {
                Some(b'"' | b'\'') => self.quoted(item)?,
                _ => self.plain(strip_comment(item).to_owned(), at + 1),
            };
            if !value.is_empty() {
                items.push(value);
            }
        }
        Ok(items)
    }

    /// The rest of the current value joined with the following lines until `done` says so.
    fn gather(&mut self, first: &str, done: impl Fn(&str) -> bool) -> Result<String, String> {
        let mut text = first.to_owned();
        while !done(&text) {
            let line = self
                .lines
                .get(self.pos)
                .ok_or("the frontmatter ends inside a value")?;
            text.push('\n');
            text.push_str(line.trim());
            self.pos += 1;
        }
        Ok(text)
    }

    fn quoted(&mut self, first: &str) -> Result<String, String> {
        let quote = first.as_bytes()[0] as char;
        let text = self.gather(first, |t| closing(t, quote).is_some())?;
        let close = closing(&text, quote).ok_or("unclosed quote")?;
        let inner = &text[1..close];
        // Line breaks inside quotes fold to spaces (blank lines stay breaks).
        let folded = inner
            .replace("\n\n", "\u{0}")
            .replace('\n', " ")
            .replace('\u{0}', "\n");
        Ok(if quote == '"' {
            unescape(&folded)
        } else {
            folded.replace("''", "'")
        })
    }

    fn flow(&mut self, first: &str) -> Result<Vec<String>, String> {
        let text = self.gather(first, |t| flow_end(t).is_some())?;
        let end = flow_end(&text).ok_or("unclosed [")?;
        Ok(split_flow(&text[1..end]))
    }
}
