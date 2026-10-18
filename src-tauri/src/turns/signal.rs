//! What one log line says about a turn, independent of the CLI that wrote it, and how a
//! tool call becomes a short activity ("run" + `npm test`, "edit" + `src/app.ts`).

use serde::Serialize;
use serde_json::Value;

/// One parsed log line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Line {
    /// ms since the epoch (0 = unknown).
    pub at: u64,
    pub cwd: Option<String>,
    pub signals: Vec<Signal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
    /// The user sent a prompt: a turn begins.
    Start,
    Think,
    Tool {
        id: String,
        action: Action,
        target: Option<String>,
    },
    ToolDone {
        id: String,
    },
    /// The CLI asks the user (approval, a question).
    Wait,
    /// Assistant text; `id` groups the blocks of one response (appended in order).
    Text {
        id: String,
        text: String,
    },
    /// Assistant text that replaces what a response said so far (Gemini re-appends
    /// messages as they grow).
    TextSet {
        id: String,
        text: String,
    },
    /// The response `id` ends the turn; its text (or `report`) is the report.
    End {
        id: Option<String>,
        report: Option<String>,
    },
    /// The CLI confirmed the end (Claude `turn_duration`).
    EndConfirmed,
    Interrupted,
}

/// What a tool call does, for the localized bubble text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Run,
    Edit,
    Read,
    Search,
    Web,
    Delegate,
    Plan,
    Tool,
}

const TARGET_MAX: usize = 160;

/// The first non-empty line of `text`, at most `TARGET_MAX` characters.
pub fn short(text: &str) -> Option<String> {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    let mut out: String = line.chars().take(TARGET_MAX).collect();
    if line.chars().count() > TARGET_MAX {
        out.push('…');
    }
    Some(out)
}

fn field(input: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| match &input[*key] {
        Value::String(text) => short(text),
        // Commands given as argv lists.
        Value::Array(items) => {
            let parts: Vec<&str> = items.iter().filter_map(Value::as_str).collect();
            short(&parts.join(" "))
        }
        _ => None,
    })
}

/// Claude Code tool names and inputs.
pub fn claude_tool(name: &str, input: &Value) -> (Action, Option<String>) {
    match name {
        "Bash" | "PowerShell" => (Action::Run, field(input, &["command"])),
        "Read" => (Action::Read, field(input, &["file_path"])),
        "Edit" | "Write" | "MultiEdit" => (Action::Edit, field(input, &["file_path"])),
        "NotebookEdit" => (Action::Edit, field(input, &["notebook_path"])),
        "Grep" | "Glob" => (Action::Search, field(input, &["pattern"])),
        "WebFetch" => (Action::Web, field(input, &["url"])),
        "WebSearch" => (Action::Web, field(input, &["query"])),
        "Task" | "Agent" => (Action::Delegate, field(input, &["description"])),
        "TodoWrite" => (Action::Plan, None),
        other => (Action::Tool, short(other)),
    }
}

/// Codex function and custom tool calls (`arguments` / `input` already decoded or raw).
pub fn codex_tool(name: &str, input: &Value) -> (Action, Option<String>) {
    match name {
        "shell" | "shell_command" | "exec_command" | "local_shell" | "container.exec" => {
            (Action::Run, field(input, &["command", "cmd"]))
        }
        "apply_patch" => (Action::Edit, input.as_str().and_then(patch_file)),
        "update_plan" => (Action::Plan, None),
        "view_image" => (Action::Read, field(input, &["path"])),
        "web_search" => (Action::Web, field(input, &["query"])),
        "exec" => match input.as_str().and_then(exec_command) {
            Some(command) => (Action::Run, Some(command)),
            None => (Action::Tool, short(name)),
        },
        other => (Action::Tool, short(other)),
    }
}

/// Gemini CLI tool names and args.
pub fn gemini_tool(name: &str, args: &Value) -> (Action, Option<String>) {
    match name {
        "run_shell_command" => (Action::Run, field(args, &["command"])),
        "read_file" | "read_many_files" => (
            Action::Read,
            field(args, &["file_path", "absolute_path", "paths"]),
        ),
        "write_file" | "replace" | "edit" => {
            (Action::Edit, field(args, &["file_path", "absolute_path"]))
        }
        "glob" | "search_file_content" | "grep" | "list_directory" => {
            (Action::Search, field(args, &["pattern", "path"]))
        }
        "web_fetch" | "google_web_search" => {
            (Action::Web, field(args, &["url", "query", "prompt"]))
        }
        "write_todos" => (Action::Plan, None),
        other => (Action::Tool, short(other)),
    }
}

/// The first file an `apply_patch` touches.
fn patch_file(patch: &str) -> Option<String> {
    const HEADS: [&str; 3] = ["*** Update File:", "*** Add File:", "*** Delete File:"];
    patch.lines().find_map(|line| {
        HEADS
            .iter()
            .find_map(|head| line.trim().strip_prefix(head))
            .and_then(short)
    })
}

/// Codex `exec` runs JavaScript that calls tools; `exec_command({cmd:"..."})` is a command.
fn exec_command(code: &str) -> Option<String> {
    let start = code.find("cmd:")? + 4;
    let rest = code[start..].trim_start();
    let quote = rest
        .chars()
        .next()
        .filter(|c| matches!(c, '"' | '\'' | '`'))?;
    let body = &rest[quote.len_utf8()..];
    let end = body.find(quote)?;
    short(&body[..end].replace("\\\"", "\""))
}

/// `target` relative to `cwd` when it lies inside it (paths read better short).
pub fn relative(target: String, cwd: Option<&str>) -> String {
    let Some(cwd) = cwd.filter(|c| !c.is_empty()) else {
        return target;
    };
    let norm = |s: &str| s.replace('\\', "/");
    let (t, c) = (norm(&target), norm(cwd));
    let c = c.trim_end_matches('/');
    let lower = cfg!(windows);
    let starts = if lower {
        t.to_lowercase()
            .starts_with(&format!("{}/", c.to_lowercase()))
    } else {
        t.starts_with(&format!("{c}/"))
    };
    match t.get(c.len() + 1..) {
        Some(rest) if starts && !rest.is_empty() => rest.to_owned(),
        _ => target,
    }
}
