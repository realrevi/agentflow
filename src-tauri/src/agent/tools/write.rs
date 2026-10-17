//! write_file and edit_file. Before the first change to a file in a turn its original (or
//! "did not exist") goes to the turn's checkpoint, so the turn can be undone.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::args::{flag, string, workspace_rel};
use super::{ToolEnv, ToolOutput};
use crate::agent::permissions::Preview;
use crate::error::AppError;
use crate::fs::{read_file, write_file as write_atomic_file};
use crate::providers::ToolCall;

/// Previews are cut here (7B renders them as a diff).
const PREVIEW_BYTES: usize = 64 * 1024;

/// (relative path, absolute target): existing files resolve (links followed, inside the
/// workspace); new files resolve through their nearest existing folder.
fn target(env: &ToolEnv, raw: &str) -> Result<(String, PathBuf), String> {
    let rel = workspace_rel(env.root.path(), raw)?;
    let path = match env.root.resolve(&rel) {
        Err(AppError::NotFound(_)) => env.root.resolve_new(&rel),
        other => other,
    }
    .map_err(|err| err.to_string())?;
    if path.is_dir() {
        return Err(format!("{raw} is a folder."));
    }
    let rel = env.root.relative(&path).unwrap_or(rel);
    Ok((rel, path))
}

/// The current text of a file: `Ok(None)` when it does not exist.
fn current_text(path: &Path, raw: &str) -> Result<Option<String>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let file = read_file(path).map_err(|err| err.to_string())?;
    match file.content {
        Some(text) if !file.binary => Ok(Some(text)),
        _ => Err(format!("{raw} is a binary file.")),
    }
}

fn save(env: &ToolEnv, rel: &str, path: &Path, content: &str) -> Result<(), String> {
    env.checkpoint
        .save_original(rel, path)
        .map_err(|err| format!("Could not save the checkpoint: {err}"))?;
    write_atomic_file(path, content.as_bytes(), None)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

pub fn write_file(env: &ToolEnv, input: &Value) -> ToolOutput {
    let (raw, content) = match (string(input, "path"), string(input, "content")) {
        (Ok(raw), Ok(content)) => (raw, content),
        (Err(err), _) | (_, Err(err)) => return ToolOutput::error(err),
    };
    let (rel, path) = match target(env, &raw) {
        Ok(target) => target,
        Err(err) => return ToolOutput::error(err),
    };
    let existed = path.exists();
    if let Err(err) = save(env, &rel, &path, &content) {
        return ToolOutput::error(err);
    }
    let lines = content.lines().count();
    let verb = if existed { "Replaced" } else { "Created" };
    ToolOutput::ok(
        format!("{verb} {rel} ({lines} lines)."),
        format!("{verb} {rel}"),
    )
}

/// The new content of an edit, or why it cannot be applied.
fn apply_edit(text: &str, input: &Value) -> Result<(String, usize), String> {
    let old = string(input, "old_string")?;
    let new = string(input, "new_string")?;
    if old.is_empty() {
        return Err("old_string is empty: use write_file to create a file.".into());
    }
    if old == new {
        return Err("old_string and new_string are the same.".into());
    }
    let count = text.matches(old.as_str()).count();
    match count {
        0 => Err(
            "old_string was not found in the file. Read the file and copy the exact \
                  text, including whitespace."
                .into(),
        ),
        1 => Ok((text.replacen(old.as_str(), &new, 1), 1)),
        n if flag(input, "replace_all") => Ok((text.replace(old.as_str(), &new), n)),
        n => Err(format!(
            "old_string appears {n} times. Include more surrounding text to make it \
             unique, or set replace_all."
        )),
    }
}

pub fn edit_file(env: &ToolEnv, input: &Value) -> ToolOutput {
    let raw = match string(input, "path") {
        Ok(raw) => raw,
        Err(err) => return ToolOutput::error(err),
    };
    let (rel, path) = match target(env, &raw) {
        Ok(target) => target,
        Err(err) => return ToolOutput::error(err),
    };
    let text = match current_text(&path, &raw) {
        Ok(Some(text)) => text,
        Ok(None) => return ToolOutput::error(format!("{raw} does not exist: use write_file.")),
        Err(err) => return ToolOutput::error(err),
    };
    let (edited, count) = match apply_edit(&text, input) {
        Ok(result) => result,
        Err(err) => return ToolOutput::error(err),
    };
    if let Err(err) = save(env, &rel, &path, &edited) {
        return ToolOutput::error(err);
    }
    let what = if count == 1 {
        "1 replacement".to_owned()
    } else {
        format!("{count} replacements")
    };
    ToolOutput::ok(format!("Edited {rel} ({what})."), format!("Edited {rel}"))
}

/// Before/after for the approval prompt; `None` when the call cannot apply anyway (its
/// error then shows after the run).
pub fn preview(env: &ToolEnv, call: &ToolCall) -> Option<Preview> {
    let raw = string(&call.input, "path").ok()?;
    let (rel, path) = target(env, &raw).ok()?;
    let before = current_text(&path, &raw).ok()?;
    let after = match call.name.as_str() {
        "write_file" => string(&call.input, "content").ok()?,
        _ => apply_edit(before.as_deref()?, &call.input).ok()?.0,
    };
    let cut = |text: String| super::truncate(&text, PREVIEW_BYTES);
    Some(Preview {
        path: rel,
        before: before.map(cut),
        after: cut(after),
    })
}
