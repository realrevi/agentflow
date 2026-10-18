//! Log line -> turn signals, per CLI (record shapes: docs/turn-signals.md). A line that is
//! not JSON is an `Err`; anything else that says nothing about a turn has no signals.

use serde_json::Value;

use super::signal::{claude_tool, codex_tool, gemini_tool, Action, Line, Signal};
use crate::tokens::cli::CliKind;
use crate::tokens::time::parse_iso;

fn text(value: &Value) -> Option<String> {
    value.as_str().filter(|s| !s.is_empty()).map(str::to_owned)
}

fn time(value: &Value) -> u64 {
    value.as_str().and_then(parse_iso).unwrap_or(0)
}

pub fn line(kind: CliKind, raw: &str) -> Result<Line, serde_json::Error> {
    let v: Value = serde_json::from_str(raw)?;
    Ok(match kind {
        CliKind::Claude => claude(&v),
        CliKind::Codex => codex(&v),
        CliKind::Gemini => gemini(&v),
    })
}

const INTERRUPTED: &str = "[Request interrupted by user";
const END_REASONS: [&str; 4] = ["end_turn", "max_tokens", "stop_sequence", "refusal"];

/// What a user prompt means: a new turn, an interrupt, or nothing (local commands).
fn user_text(text: &str, signals: &mut Vec<Signal>) {
    let text = text.trim_start();
    if text.starts_with(INTERRUPTED) {
        signals.push(Signal::Interrupted);
    } else if !text.is_empty() && !text.starts_with("<command-") && !text.starts_with("<local-") {
        signals.push(Signal::Start);
    }
}

fn claude(v: &Value) -> Line {
    let mut line = Line {
        at: time(&v["timestamp"]),
        cwd: text(&v["cwd"]),
        signals: Vec::new(),
    };
    if v["isSidechain"] == true || v["isMeta"] == true {
        return line;
    }
    let message = &v["message"];
    let signals = &mut line.signals;
    match v["type"].as_str().unwrap_or("") {
        "assistant" if message["model"] != "<synthetic>" => {
            let id = text(&message["id"]).unwrap_or_default();
            for block in message["content"].as_array().into_iter().flatten() {
                match block["type"].as_str().unwrap_or("") {
                    "thinking" | "redacted_thinking" => signals.push(Signal::Think),
                    "text" => signals.push(Signal::Text {
                        id: id.clone(),
                        text: block["text"].as_str().unwrap_or("").to_owned(),
                    }),
                    "tool_use" => {
                        let name = block["name"].as_str().unwrap_or("");
                        let (action, target) = claude_tool(name, &block["input"]);
                        signals.push(Signal::Tool {
                            id: text(&block["id"]).unwrap_or_default(),
                            action,
                            target,
                        });
                    }
                    _ => {}
                }
            }
            let reason = message["stop_reason"].as_str().unwrap_or("");
            if END_REASONS.contains(&reason) {
                signals.push(Signal::End {
                    id: Some(id),
                    report: None,
                });
            }
        }
        "user" => match &message["content"] {
            Value::String(prompt) => user_text(prompt, signals),
            Value::Array(blocks) => {
                let mut results = false;
                for block in blocks {
                    if block["type"] == "tool_result" {
                        results = true;
                        let id = text(&block["tool_use_id"]).unwrap_or_default();
                        signals.push(Signal::ToolDone { id });
                    }
                }
                for block in blocks.iter().filter(|b| b["type"] == "text") {
                    let prompt = block["text"].as_str().unwrap_or("");
                    if results && !prompt.starts_with(INTERRUPTED) {
                        continue;
                    }
                    user_text(prompt, signals);
                }
            }
            _ => {}
        },
        "system" if v["subtype"] == "turn_duration" => signals.push(Signal::EndConfirmed),
        _ => {}
    }
    line
}

fn codex(v: &Value) -> Line {
    let payload = &v["payload"];
    let mut line = Line {
        at: time(&v["timestamp"]),
        cwd: None,
        signals: Vec::new(),
    };
    let call_id = || text(&payload["call_id"]).unwrap_or_default();
    let kind = payload["type"].as_str().unwrap_or("");
    let signal = match (v["type"].as_str().unwrap_or(""), kind) {
        ("session_meta" | "turn_context", _) => {
            line.cwd = text(&payload["cwd"]);
            None
        }
        ("event_msg", "task_started" | "turn_started") => Some(Signal::Start),
        ("event_msg", "task_complete" | "turn_complete") => Some(Signal::End {
            id: None,
            report: text(&payload["last_agent_message"]),
        }),
        ("event_msg", "turn_aborted") => Some(Signal::Interrupted),
        ("event_msg", "exec_approval_request" | "apply_patch_approval_request") => {
            Some(Signal::Wait)
        }
        ("response_item", "reasoning") => Some(Signal::Think),
        ("response_item", "function_call") => {
            let name = payload["name"].as_str().unwrap_or("");
            let raw = payload["arguments"].as_str().unwrap_or("");
            let args = serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.into()));
            if name.starts_with("request_user_input") {
                Some(Signal::Wait)
            } else {
                let (action, target) = codex_tool(name, &args);
                Some(Signal::Tool {
                    id: call_id(),
                    action,
                    target,
                })
            }
        }
        ("response_item", "custom_tool_call") => {
            let name = payload["name"].as_str().unwrap_or("");
            let (action, target) = codex_tool(name, &payload["input"]);
            Some(Signal::Tool {
                id: call_id(),
                action,
                target,
            })
        }
        ("response_item", "local_shell_call") => {
            let (_, target) = codex_tool("local_shell", &payload["action"]);
            Some(Signal::Tool {
                id: call_id(),
                action: Action::Run,
                target,
            })
        }
        (
            "response_item",
            "function_call_output" | "custom_tool_call_output" | "local_shell_call_output",
        ) => Some(Signal::ToolDone { id: call_id() }),
        ("response_item", "message") if payload["role"] == "assistant" => {
            let parts = payload["content"].as_array().into_iter().flatten();
            let said: Vec<&str> = parts
                .filter(|part| part["type"] == "output_text")
                .filter_map(|part| part["text"].as_str())
                .collect();
            Some(Signal::TextSet {
                id: text(&payload["id"]).unwrap_or_default(),
                text: said.join(""),
            })
        }
        _ => None,
    };
    line.signals.extend(signal);
    line
}

/// Gemini `content` is a string or a list of parts with `text`.
fn gemini_content(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part["text"].as_str())
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

fn gemini(v: &Value) -> Line {
    let mut line = Line {
        at: time(&v["timestamp"]),
        cwd: None,
        signals: Vec::new(),
    };
    let signals = &mut line.signals;
    match v["type"].as_str().unwrap_or("") {
        "user" => user_text(&gemini_content(&v["content"]), signals),
        "gemini" => {
            let id = text(&v["id"]).unwrap_or_default();
            let said = gemini_content(&v["content"]);
            let thoughts = v["thoughts"].as_array().is_some_and(|t| !t.is_empty());
            if said.is_empty() && thoughts {
                signals.push(Signal::Think);
            }
            for call in v["toolCalls"].as_array().into_iter().flatten() {
                let call_id = text(&call["id"]).unwrap_or_default();
                match call["status"].as_str().unwrap_or("") {
                    "success" | "error" | "cancelled" => {
                        signals.push(Signal::ToolDone { id: call_id });
                    }
                    status => {
                        let name = call["name"].as_str().unwrap_or("");
                        let (action, target) = gemini_tool(name, &call["args"]);
                        signals.push(Signal::Tool {
                            id: call_id,
                            action,
                            target,
                        });
                        if status == "awaiting_approval" {
                            signals.push(Signal::Wait);
                        }
                    }
                }
            }
            if !said.is_empty() {
                signals.push(Signal::TextSet { id, text: said });
            }
        }
        _ => {}
    }
    line
}
