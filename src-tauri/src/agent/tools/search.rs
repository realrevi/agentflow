//! glob and search over the workspace walker (.gitignore respected), with capped results.

use globset::{Glob, GlobMatcher};
use regex::Regex;
use serde_json::Value;

use super::args::{optional_string, string, workspace_rel};
use super::{ToolEnv, ToolOutput};
use crate::fs::walker;

const MAX_FILES: usize = 200;
const MAX_MATCHES: usize = 200;
/// Files bigger than this are not searched (minified bundles, data dumps).
const MAX_SEARCH_BYTES: u64 = 1024 * 1024;
const MAX_LINE_CHARS: usize = 200;

fn matcher(pattern: &str) -> Result<GlobMatcher, String> {
    Glob::new(pattern)
        .map(|glob| glob.compile_matcher())
        .map_err(|err| format!("Invalid glob \"{pattern}\": {err}"))
}

/// Workspace-relative files (with `/`) under `rel`, in walker order.
fn files(env: &ToolEnv, rel: &str) -> Result<Vec<String>, String> {
    let start = env.root.resolve(rel).map_err(|err| err.to_string())?;
    let mut out = Vec::new();
    for entry in walker(&start, false).build().flatten() {
        if entry.file_type().is_some_and(|t| t.is_file()) {
            if let Some(rel) = env.root.relative(entry.path()) {
                out.push(rel);
            }
        }
    }
    Ok(out)
}

pub fn glob(env: &ToolEnv, input: &Value) -> ToolOutput {
    let pattern = match string(input, "pattern") {
        Ok(pattern) => pattern,
        Err(err) => return ToolOutput::error(err),
    };
    let matcher = match matcher(&pattern) {
        Ok(matcher) => matcher,
        Err(err) => return ToolOutput::error(err),
    };
    let mut found = match files(env, ".") {
        Ok(files) => files,
        Err(err) => return ToolOutput::error(err),
    };
    found.retain(|path| matcher.is_match(path));
    found.sort();
    let total = found.len();
    found.truncate(MAX_FILES);
    let mut out = found.join("\n");
    if total > MAX_FILES {
        out.push_str(&format!("\n… and {} more", total - MAX_FILES));
    }
    if total == 0 {
        out = "No files match.".into();
    }
    ToolOutput::ok(out, format!("{pattern}: {total} files"))
}

pub fn search(env: &ToolEnv, input: &Value) -> ToolOutput {
    let pattern = match string(input, "regex") {
        Ok(pattern) => pattern,
        Err(err) => return ToolOutput::error(err),
    };
    let regex = match Regex::new(&pattern) {
        Ok(regex) => regex,
        Err(err) => return ToolOutput::error(format!("Invalid regex: {err}")),
    };
    let raw_path = optional_string(input, "path").unwrap_or_else(|| ".".into());
    let rel = match workspace_rel(env.root.path(), &raw_path) {
        Ok(rel) => rel,
        Err(err) => return ToolOutput::error(err),
    };
    let only = match optional_string(input, "glob")
        .map(|g| matcher(&g))
        .transpose()
    {
        Ok(only) => only,
        Err(err) => return ToolOutput::error(err),
    };
    let candidates = match env.root.resolve(&rel) {
        Ok(path) if path.is_file() => vec![env.root.relative(&path).unwrap_or(rel)],
        Ok(_) => match files(env, &rel) {
            Ok(files) => files,
            Err(err) => return ToolOutput::error(err),
        },
        Err(err) => return ToolOutput::error(err.to_string()),
    };
    let mut lines: Vec<String> = Vec::new();
    let mut total = 0usize;
    let mut files_hit = 0usize;
    for file in candidates {
        if only.as_ref().is_some_and(|m| !m.is_match(&file)) {
            continue;
        }
        let Ok(path) = env.root.resolve(&file) else {
            continue;
        };
        if std::fs::metadata(&path).map_or(true, |m| m.len() > MAX_SEARCH_BYTES) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if bytes.contains(&0) {
            continue; // binary
        }
        let text = String::from_utf8_lossy(&bytes);
        let mut hit = false;
        for (index, line) in text.lines().enumerate() {
            if !regex.is_match(line) {
                continue;
            }
            hit = true;
            total += 1;
            if lines.len() < MAX_MATCHES {
                let shown: String = line.trim_end().chars().take(MAX_LINE_CHARS).collect();
                lines.push(format!("{file}:{}: {shown}", index + 1));
            }
        }
        files_hit += usize::from(hit);
    }
    let mut out = lines.join("\n");
    if total > MAX_MATCHES {
        out.push_str(&format!("\n… and {} more matches", total - MAX_MATCHES));
    }
    if total == 0 {
        out = "No matches.".into();
    }
    ToolOutput::ok(out, format!("{total} matches in {files_hit} files"))
}
