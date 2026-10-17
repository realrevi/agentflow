//! Managed sections in shared instruction files (CLAUDE.md, AGENTS.md, GEMINI.md): the
//! installer only ever writes between `<!-- agentflow:begin <id> -->` and
//! `<!-- agentflow:end <id> -->`. Everything outside the markers stays byte for byte; the
//! file's line endings (LF or CRLF) are kept; writing the same section twice changes nothing.

use crate::error::{AppError, AppResult};

/// Templates and answers may not contain this (they could fake or break a marker).
pub const MARKER_PREFIX: &str = "<!-- agentflow:";

pub fn begin_marker(id: &str) -> String {
    format!("{MARKER_PREFIX}begin {id} -->")
}

pub fn end_marker(id: &str) -> String {
    format!("{MARKER_PREFIX}end {id} -->")
}

fn newline(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Byte range of the whole section: from the start of the begin marker's line to the end of
/// the end marker's line (including its line break, if any).
fn find(text: &str, id: &str) -> AppResult<Option<(usize, usize)>> {
    let begin = begin_marker(id);
    let end = end_marker(id);
    let Some(start) = text.find(&begin) else {
        return if text.contains(&end) {
            Err(broken(id))
        } else {
            Ok(None)
        };
    };
    if text[start + begin.len()..].contains(&begin) {
        return Err(broken(id));
    }
    let Some(offset) = text[start..].find(&end) else {
        return Err(broken(id));
    };
    let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let mut stop = start + offset + end.len();
    if text[stop..].starts_with("\r\n") {
        stop += 2;
    } else if text[stop..].starts_with('\n') {
        stop += 1;
    }
    Ok(Some((line_start, stop)))
}

fn broken(id: &str) -> AppError {
    AppError::InvalidInput(format!(
        "the AgentFlow section \"{id}\" has a missing or duplicated marker; fix it by hand first"
    ))
}

fn section(id: &str, body: &str, nl: &str) -> String {
    let body = body
        .replace("\r\n", "\n")
        .trim_matches('\n')
        .replace('\n', nl);
    format!("{}{nl}{body}{nl}{}{nl}", begin_marker(id), end_marker(id))
}

/// Inserts (at the end, after a blank line) or replaces the section.
pub fn upsert(text: &str, id: &str, body: &str) -> AppResult<String> {
    let nl = newline(text);
    let block = section(id, body, nl);
    Ok(match find(text, id)? {
        Some((start, stop)) => format!("{}{block}{}", &text[..start], &text[stop..]),
        None if text.is_empty() => block,
        None => {
            let mut out = text.to_owned();
            if !out.ends_with('\n') {
                out.push_str(nl);
            }
            out.push_str(nl);
            out.push_str(&block);
            out
        }
    })
}

/// Removes the section and the blank line `upsert` put before it; unchanged if absent.
pub fn remove(text: &str, id: &str) -> AppResult<String> {
    let Some((mut start, stop)) = find(text, id)? else {
        return Ok(text.to_owned());
    };
    let nl = newline(text);
    let at_end = text[stop..].trim().is_empty();
    if at_end && text[..start].ends_with(&format!("{nl}{nl}")) {
        start -= nl.len();
    }
    Ok(format!("{}{}", &text[..start], &text[stop..]))
}

/// The section's content (LF line endings), if present.
pub fn content(text: &str, id: &str) -> AppResult<Option<String>> {
    Ok(find(text, id)?.map(|(start, stop)| {
        let inner = &text[start..stop];
        let after_begin = inner.find("-->").map_or(0, |i| i + 3);
        let before_end = inner.rfind(MARKER_PREFIX).unwrap_or(inner.len());
        inner[after_begin..before_end]
            .replace("\r\n", "\n")
            .trim_matches('\n')
            .to_owned()
    }))
}
