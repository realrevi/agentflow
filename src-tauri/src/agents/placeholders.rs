//! `{{placeholder}}` rendering: plain substitution, nothing is evaluated. A placeholder
//! names a question (its answer) or a built-in (`name`, `description`, `workspaceName`).
//! Unknown names are a validation error; a required question without an answer is a render
//! error. A line whose placeholders all come out empty is dropped, so optional answers leave
//! no "Colors: " stubs behind.

use std::collections::BTreeMap;

use serde_json::Value;

use super::template::{Answers, QuestionKind, Template};
use super::validate::fits;
use crate::error::{AppError, AppResult};

pub const BUILT_INS: [&str; 3] = ["name", "description", "workspaceName"];

/// `\{{` is a literal `{{` (imported text keeps its own braces this way).
pub const ESCAPE: &str = "\\{{";

/// Escapes every `{{` of `text`, so it renders back to exactly `text`.
pub fn escape(text: &str) -> String {
    text.replace("{{", ESCAPE)
}

fn unescape(text: &str) -> String {
    text.replace(ESCAPE, "{{")
}

/// `{{ name }}` occurrences: (start byte, end byte, name). A name is a letter followed by
/// letters, digits or `_`; anything else between braces is left as text, and so is an
/// escaped `\{{`.
fn scan(text: &str) -> Vec<(usize, usize, &str)> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(offset) = text[from..].find("{{") {
        let start = from + offset;
        let inner_start = start + 2;
        if start > 0 && text.as_bytes()[start - 1] == b'\\' {
            from = inner_start;
            continue;
        }
        let Some(close) = text[inner_start..].find("}}") else {
            break;
        };
        let inner = &text[inner_start..inner_start + close];
        let name = inner.trim();
        let valid = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if valid && !inner.contains('\n') {
            let end = inner_start + close + 2;
            found.push((start, end, name));
            from = end;
        } else {
            from = inner_start;
        }
    }
    found
}

/// The placeholder names used in `text`, in order (with repeats).
pub fn placeholders(text: &str) -> Vec<String> {
    scan(text)
        .into_iter()
        .map(|(_, _, name)| name.to_owned())
        .collect()
}

/// What answers turn into in text.
fn text_of(value: &Value) -> String {
    match value {
        Value::String(s) => s.trim().to_owned(),
        Value::Bool(true) => "yes".into(),
        Value::Bool(false) => "no".into(),
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Number(n) => n.to_string(),
        Value::Null | Value::Object(_) => String::new(),
    }
}

/// The value of every placeholder for these answers: the answer, else the default, else
/// empty (an error for required questions). Answers of the wrong shape are rejected.
pub fn values(
    template: &Template,
    answers: &Answers,
    workspace_name: &str,
) -> AppResult<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    out.insert("name".into(), template.name.trim().to_owned());
    out.insert("description".into(), template.description.trim().to_owned());
    out.insert("workspaceName".into(), workspace_name.to_owned());
    for question in &template.questions {
        let answer = answers.get(&question.id).filter(|v| !v.is_null());
        if let Some(value) = answer {
            if !fits(question.kind, value, &question.options) {
                return Err(AppError::InvalidInput(format!(
                    "the answer to \"{}\" does not fit the question",
                    question.label
                )));
            }
        }
        let text = answer
            .or(question.default.as_ref())
            .map(text_of)
            .unwrap_or_default();
        if text.is_empty() && question.required && question.kind != QuestionKind::Boolean {
            return Err(AppError::InvalidInput(format!(
                "an answer is needed for \"{}\"",
                question.label
            )));
        }
        out.insert(question.id.clone(), text);
    }
    Ok(out)
}

/// Renders `text`; every placeholder must be in `values`.
pub fn render(text: &str, values: &BTreeMap<String, String>) -> AppResult<String> {
    let mut lines = Vec::new();
    for line in text.split('\n') {
        let found = scan(line);
        let mut out = String::with_capacity(line.len());
        let mut last = 0;
        let mut all_empty = !found.is_empty();
        for (start, end, name) in found {
            let value = values.get(name).ok_or_else(|| {
                AppError::InvalidInput(format!("unknown placeholder {{{{{name}}}}}"))
            })?;
            all_empty &= value.is_empty();
            out.push_str(&unescape(&line[last..start]));
            out.push_str(value);
            last = end;
        }
        out.push_str(&unescape(&line[last..]));
        if !all_empty {
            lines.push(out);
        }
    }
    Ok(lines.join("\n"))
}
