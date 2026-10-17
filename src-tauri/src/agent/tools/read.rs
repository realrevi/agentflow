//! list_dir and read_file: the workspace walker (.gitignore respected, `.git` hidden) and
//! the editor's read rules (5 MB limit, binary files refused).

use serde_json::Value;

use super::args::{display_path, optional_string, optional_u64, string, workspace_rel};
use super::{ToolEnv, ToolOutput};
use crate::fs::{read_file as read_text, walker};

const MAX_ENTRIES: usize = 500;
const DEFAULT_LINES: u64 = 2_000;

pub fn list_dir(env: &ToolEnv, input: &Value) -> ToolOutput {
    let raw = optional_string(input, "path").unwrap_or_else(|| ".".into());
    let depth = optional_u64(input, "depth").unwrap_or(1).clamp(1, 3) as usize;
    let result = workspace_rel(env.root.path(), &raw)
        .and_then(|rel| env.root.resolve(&rel).map_err(|err| err.to_string()));
    let dir = match result {
        Ok(dir) if dir.is_dir() => dir,
        Ok(_) => return ToolOutput::error(format!("{raw} is not a folder.")),
        Err(err) => return ToolOutput::error(err),
    };
    let mut entries: Vec<String> = Vec::new();
    let mut more = 0usize;
    for entry in walker(&dir, false).max_depth(Some(depth)).build().flatten() {
        if entry.depth() == 0 {
            continue;
        }
        let Some(rel) = env.root.relative(entry.path()) else {
            continue;
        };
        if entries.len() >= MAX_ENTRIES {
            more += 1;
            continue;
        }
        let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
        entries.push(if is_dir { format!("{rel}/") } else { rel });
    }
    entries.sort();
    let count = entries.len() + more;
    let mut out = entries.join("\n");
    if more > 0 {
        out.push_str(&format!("\n… and {more} more"));
    }
    if count == 0 {
        out = "(empty)".into();
    }
    let rel = env.root.relative(&dir).unwrap_or_default();
    ToolOutput::ok(out, format!("{}: {count} entries", display_path(&rel)))
}

pub fn read_file(env: &ToolEnv, input: &Value) -> ToolOutput {
    let raw = match string(input, "path") {
        Ok(raw) => raw,
        Err(err) => return ToolOutput::error(err),
    };
    let path = match workspace_rel(env.root.path(), &raw)
        .and_then(|rel| env.root.resolve(&rel).map_err(|err| err.to_string()))
    {
        Ok(path) => path,
        Err(err) => return ToolOutput::error(err),
    };
    if path.is_dir() {
        return ToolOutput::error(format!("{raw} is a folder: use list_dir."));
    }
    let file = match read_text(&path) {
        Ok(file) => file,
        Err(err) => return ToolOutput::error(err.to_string()),
    };
    let Some(text) = file.content.filter(|_| !file.binary) else {
        return ToolOutput::error(format!("{raw} is a binary file."));
    };
    let first = optional_u64(input, "offset").unwrap_or(1).max(1);
    let limit = optional_u64(input, "limit").unwrap_or(DEFAULT_LINES).max(1);
    let total = text.lines().count() as u64;
    let mut out = String::new();
    for (index, line) in text
        .lines()
        .enumerate()
        .skip((first - 1) as usize)
        .take(limit as usize)
    {
        out.push_str(&format!("{:>6}\t{line}\n", index + 1));
    }
    let last = (first + limit - 1).min(total);
    if last < total {
        out.push_str(&format!(
            "… {} more lines (read on with offset {})\n",
            total - last,
            last + 1
        ));
    }
    if total == 0 {
        out = "(empty file)".into();
    }
    let rel = env.root.relative(&path).unwrap_or(raw);
    let summary = if first == 1 && last >= total {
        format!("{rel}: {total} lines")
    } else {
        format!("{rel}: lines {first}–{last} of {total}")
    };
    ToolOutput::ok(out, summary)
}
