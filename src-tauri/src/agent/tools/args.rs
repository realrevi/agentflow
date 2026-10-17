//! Reading tool arguments and turning the model's paths into workspace-relative ones.

use std::path::{Path, PathBuf};

use serde_json::Value;

pub fn string(input: &Value, key: &str) -> Result<String, String> {
    input[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("Missing \"{key}\" (a string)."))
}

pub fn optional_string(input: &Value, key: &str) -> Option<String> {
    input[key]
        .as_str()
        .map(str::to_owned)
        .filter(|s| !s.is_empty())
}

pub fn optional_u64(input: &Value, key: &str) -> Option<u64> {
    match &input[key] {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

pub fn flag(input: &Value, key: &str) -> bool {
    match &input[key] {
        Value::Bool(b) => *b,
        Value::String(s) => s.eq_ignore_ascii_case("true"),
        _ => false,
    }
}

/// A path relative to the workspace with `/`. Absolute paths are accepted only inside the
/// workspace; `..` and anything resolving outside is rejected later by the sandbox.
pub fn workspace_rel(root: &Path, raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    let path = PathBuf::from(raw);
    if path.is_absolute() || raw.starts_with('/') || raw.starts_with('\\') {
        let canonical = std::fs::canonicalize(&path).unwrap_or(path);
        let rest = canonical
            .strip_prefix(root)
            .map_err(|_| format!("{raw} is outside the workspace."))?;
        return Ok(rest.to_string_lossy().replace('\\', "/"));
    }
    Ok(raw.replace('\\', "/"))
}
