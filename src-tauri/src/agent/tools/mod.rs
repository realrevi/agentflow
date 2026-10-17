//! The agent's tools: definitions (name, category, JSON Schema), dispatch, status text for
//! the character's bubble and before/after previews for approvals. Every path goes through
//! the workspace sandbox (`fs::WorkspaceRoot`: canonicalized, escapes rejected).

mod args;
mod command;
mod read;
mod search;
mod write;

use std::time::Duration;

use serde_json::{json, Value};

use super::checkpoint::TurnCheckpoint;
use super::permissions::{Category, Preview};
use crate::conversations::PermissionMode;
use crate::fs::WorkspaceRoot;
use crate::providers::{CancelToken, ToolCall, ToolSpec};

/// What a tool run needs: the sandboxed workspace, limits, the turn's checkpoint.
pub struct ToolEnv {
    pub root: WorkspaceRoot,
    /// Output sent to the model is cut at this many bytes.
    pub cap: usize,
    pub timeout: Duration,
    pub checkpoint: TurnCheckpoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutput {
    pub content: String,
    pub is_error: bool,
    /// One line for the chat ("12 files", "exit 0", "3 matches").
    pub summary: String,
}

impl ToolOutput {
    pub fn ok(content: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: false,
            summary: summary.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            summary: message
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(120)
                .collect(),
            content: message,
            is_error: true,
        }
    }
}

struct ToolDef {
    name: &'static str,
    category: Category,
    description: &'static str,
    schema: fn() -> Value,
}

const TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "list_dir",
        category: Category::Read,
        description: "List a folder of the workspace (.gitignore respected). Folders end with /.",
        schema: || {
            object(
                json!({
                    "path": { "type": "string", "description": "Folder, relative to the workspace (default \".\")" },
                    "depth": { "type": "integer", "minimum": 1, "maximum": 3, "description": "Levels to list (default 1)" }
                }),
                &[],
            )
        },
    },
    ToolDef {
        name: "read_file",
        category: Category::Read,
        description: "Read a text file with line numbers. Use offset/limit for long files.",
        schema: || {
            object(
                json!({
                    "path": { "type": "string" },
                    "offset": { "type": "integer", "minimum": 1, "description": "First line (1-based)" },
                    "limit": { "type": "integer", "minimum": 1, "description": "Number of lines (default 2000)" }
                }),
                &["path"],
            )
        },
    },
    ToolDef {
        name: "glob",
        category: Category::Read,
        description: "Find files by glob pattern, e.g. \"src/**/*.ts\" (.gitignore respected).",
        schema: || object(json!({ "pattern": { "type": "string" } }), &["pattern"]),
    },
    ToolDef {
        name: "search",
        category: Category::Read,
        description: "Search file contents with a regular expression; returns path:line: text.",
        schema: || {
            object(
                json!({
                    "regex": { "type": "string" },
                    "path": { "type": "string", "description": "Folder or file to search (default: the workspace)" },
                    "glob": { "type": "string", "description": "Only files matching this glob" }
                }),
                &["regex"],
            )
        },
    },
    ToolDef {
        name: "write_file",
        category: Category::Write,
        description: "Create a file or replace its whole content. Prefer edit_file for changes.",
        schema: || {
            object(
                json!({ "path": { "type": "string" }, "content": { "type": "string" } }),
                &["path", "content"],
            )
        },
    },
    ToolDef {
        name: "edit_file",
        category: Category::Write,
        description: "Replace an exact string in a file. old_string must match exactly once \
                      (include surrounding lines to make it unique) unless replace_all is true.",
        schema: || {
            object(
                json!({
                    "path": { "type": "string" },
                    "old_string": { "type": "string" },
                    "new_string": { "type": "string" },
                    "replace_all": { "type": "boolean" }
                }),
                &["path", "old_string", "new_string"],
            )
        },
    },
    ToolDef {
        name: "run_command",
        category: Category::Command,
        description: "Run a shell command in the workspace (non-interactive: stdin is closed). \
                      Returns the exit code, stdout and stderr.",
        schema: || {
            object(
                json!({
                    "command": { "type": "string" },
                    "timeout": { "type": "integer", "minimum": 1, "description": "Seconds (default 120)" }
                }),
                &["command"],
            )
        },
    },
];

fn object(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required })
}

/// The tools offered in `mode` (Read-only offers the read tools only).
pub fn specs(mode: PermissionMode) -> Vec<ToolSpec> {
    TOOLS
        .iter()
        .filter(|tool| mode != PermissionMode::ReadOnly || tool.category == Category::Read)
        .map(|tool| ToolSpec {
            name: tool.name.into(),
            description: tool.description.into(),
            schema: (tool.schema)(),
        })
        .collect()
}

pub fn category(name: &str) -> Option<Category> {
    TOOLS.iter().find(|t| t.name == name).map(|t| t.category)
}

/// For the speech bubble: "Reading src/app.ts", "Running npm test".
pub fn status_text(call: &ToolCall) -> String {
    let arg = |key: &str| {
        call.input[key]
            .as_str()
            .map(|s| s.chars().take(60).collect::<String>())
    };
    match call.name.as_str() {
        "list_dir" => format!("Listing {}", arg("path").unwrap_or_else(|| ".".into())),
        "read_file" => format!("Reading {}", arg("path").unwrap_or_default()),
        "glob" => format!("Finding {}", arg("pattern").unwrap_or_default()),
        "search" => format!("Searching {}", arg("regex").unwrap_or_default()),
        "write_file" => format!("Writing {}", arg("path").unwrap_or_default()),
        "edit_file" => format!("Editing {}", arg("path").unwrap_or_default()),
        "run_command" => format!("Running {}", arg("command").unwrap_or_default()),
        other => format!("Using {other}"),
    }
}

/// Runs one call. Errors become error results the model can react to.
pub async fn run(env: &ToolEnv, call: &ToolCall, cancel: &CancelToken) -> ToolOutput {
    if let Some(raw) = call.input.get("_raw") {
        return ToolOutput::error(format!(
            "The arguments were not valid JSON: {}",
            raw.as_str().unwrap_or("")
        ));
    }
    let output = match call.name.as_str() {
        "list_dir" => blocking(env, call, read::list_dir).await,
        "read_file" => blocking(env, call, read::read_file).await,
        "glob" => blocking(env, call, search::glob).await,
        "search" => blocking(env, call, search::search).await,
        "write_file" => blocking(env, call, write::write_file).await,
        "edit_file" => blocking(env, call, write::edit_file).await,
        "run_command" => command::run_command(env, &call.input, cancel).await,
        other => ToolOutput::error(format!("Unknown tool \"{other}\".")),
    };
    ToolOutput {
        content: truncate(&output.content, env.cap),
        ..output
    }
}

/// File tools do blocking I/O: off the async runtime.
async fn blocking(
    env: &ToolEnv,
    call: &ToolCall,
    tool: fn(&ToolEnv, &Value) -> ToolOutput,
) -> ToolOutput {
    let (root, checkpoint) = (env.root.clone(), env.checkpoint.clone());
    let (cap, timeout, input) = (env.cap, env.timeout, call.input.clone());
    let task = tauri::async_runtime::spawn_blocking(move || {
        let env = ToolEnv {
            root,
            cap,
            timeout,
            checkpoint,
        };
        tool(&env, &input)
    });
    task.await
        .unwrap_or_else(|err| ToolOutput::error(format!("The tool failed: {err}")))
}

/// Before/after for write and edit approvals.
pub fn preview(env: &ToolEnv, call: &ToolCall) -> Option<Preview> {
    match call.name.as_str() {
        "write_file" | "edit_file" => write::preview(env, call),
        _ => None,
    }
}

/// Cuts `text` at `cap` bytes (on a char boundary) with a note of what was left out.
pub fn truncate(text: &str, cap: usize) -> String {
    if text.len() <= cap {
        return text.to_owned();
    }
    let mut cut = cap;
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    format!(
        "{}\n[truncated, {} more bytes]",
        &text[..cut],
        text.len() - cut
    )
}

#[cfg(test)]
mod tests;
