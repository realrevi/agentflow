//! Which tool calls run on their own and which ask the user, per permission mode:
//! Ask (reads run; writes and commands ask), Auto-edit (reads and writes run; commands
//! ask), Read-only (only read tools are offered at all). "Allow for this conversation" lets
//! a tool run from then on, except dangerous commands, which always ask.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::oneshot;

use crate::conversations::PermissionMode;
use crate::util::lock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Read,
    Write,
    Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Run,
    Ask,
    /// Not allowed in this mode (the model should not even have the tool).
    Refuse,
}

/// `dangerous` = the command matched `dangerous.rs` (always asks).
pub fn decide(
    mode: PermissionMode,
    category: Category,
    tool: &str,
    allowed: &[String],
    dangerous: bool,
) -> Decision {
    let allowed_here = allowed.iter().any(|name| name == tool) && !dangerous;
    match (mode, category) {
        (_, Category::Read) => Decision::Run,
        (PermissionMode::ReadOnly, _) => Decision::Refuse,
        (PermissionMode::AutoEdit, Category::Write) => Decision::Run,
        (_, Category::Command) if dangerous => Decision::Ask,
        _ if allowed_here => Decision::Run,
        _ => Decision::Ask,
    }
}

/// Before/after of a write or edit, for the approval prompt (7B shows it as a diff).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub path: String,
    /// `None` = the file does not exist yet.
    pub before: Option<String>,
    pub after: String,
}

/// What the chat panel shows while the agent waits for the user.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequest {
    /// Answer with `agent_permission_answer(id, ...)`.
    pub id: u64,
    pub call_id: String,
    pub tool: String,
    pub input: Value,
    pub category: Category,
    /// Why the command counts as dangerous (it asks even when the tool is allowed).
    pub dangerous: Option<String>,
    pub preview: Option<Preview>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "decision", rename_all = "camelCase")]
pub enum PermissionAnswer {
    AllowOnce,
    /// Allow this tool for the rest of the conversation.
    AllowConversation,
    /// Optional reason, sent to the model with the refusal.
    Deny {
        reason: Option<String>,
    },
}

/// Managed state: permission requests waiting for an answer from the frontend.
#[derive(Clone, Default)]
pub struct PermissionGate {
    waiting: Arc<Mutex<HashMap<u64, oneshot::Sender<PermissionAnswer>>>>,
    next: Arc<AtomicU64>,
}

impl PermissionGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// A new request id and the receiver its answer arrives on.
    pub fn open(&self) -> (u64, oneshot::Receiver<PermissionAnswer>) {
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        let (tx, rx) = oneshot::channel();
        lock(&self.waiting).insert(id, tx);
        (id, rx)
    }

    /// Delivers an answer; `false` if nobody waits for `id` any more.
    pub fn answer(&self, id: u64, answer: PermissionAnswer) -> bool {
        match lock(&self.waiting).remove(&id) {
            Some(tx) => tx.send(answer).is_ok(),
            None => false,
        }
    }

    /// Forgets a request whose turn ended (cancelled) without an answer.
    pub fn close(&self, id: u64) {
        lock(&self.waiting).remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Category::*;
    use PermissionMode::*;

    #[test]
    fn modes_decide_per_category() {
        let none: Vec<String> = Vec::new();
        assert_eq!(decide(Ask, Read, "read_file", &none, false), Decision::Run);
        assert_eq!(
            decide(Ask, Write, "write_file", &none, false),
            Decision::Ask
        );
        assert_eq!(
            decide(Ask, Command, "run_command", &none, false),
            Decision::Ask
        );
        assert_eq!(
            decide(AutoEdit, Write, "edit_file", &none, false),
            Decision::Run
        );
        assert_eq!(
            decide(AutoEdit, Command, "run_command", &none, false),
            Decision::Ask
        );
        assert_eq!(
            decide(ReadOnly, Read, "search", &none, false),
            Decision::Run
        );
        assert_eq!(
            decide(ReadOnly, Write, "write_file", &none, false),
            Decision::Refuse
        );
        assert_eq!(
            decide(ReadOnly, Command, "run_command", &none, false),
            Decision::Refuse
        );
    }

    #[test]
    fn allowed_for_the_conversation_except_dangerous_commands() {
        let allowed = vec!["run_command".to_owned(), "write_file".to_owned()];
        assert_eq!(
            decide(Ask, Command, "run_command", &allowed, false),
            Decision::Run
        );
        assert_eq!(
            decide(Ask, Write, "write_file", &allowed, false),
            Decision::Run
        );
        assert_eq!(
            decide(Ask, Write, "edit_file", &allowed, false),
            Decision::Ask
        );
        assert_eq!(
            decide(Ask, Command, "run_command", &allowed, true),
            Decision::Ask
        );
        assert_eq!(
            decide(AutoEdit, Command, "run_command", &allowed, true),
            Decision::Ask
        );
    }

    #[test]
    fn answers_reach_the_waiting_request_once() {
        let gate = PermissionGate::new();
        let (id, rx) = gate.open();
        assert!(gate.answer(id, PermissionAnswer::AllowOnce));
        assert!(
            !gate.answer(id, PermissionAnswer::AllowOnce),
            "already answered"
        );
        let answer = tauri::async_runtime::block_on(rx).expect("answer");
        assert_eq!(answer, PermissionAnswer::AllowOnce);
        let (closed, _rx) = gate.open();
        gate.close(closed);
        assert!(!gate.answer(closed, PermissionAnswer::Deny { reason: None }));
    }

    #[test]
    fn answers_parse_from_the_frontend_shape() {
        let deny: PermissionAnswer =
            serde_json::from_str(r#"{"decision":"deny","reason":"use npm ci"}"#).expect("json");
        assert_eq!(
            deny,
            PermissionAnswer::Deny {
                reason: Some("use npm ci".into())
            }
        );
        let all: PermissionAnswer =
            serde_json::from_str(r#"{"decision":"allowConversation"}"#).expect("json");
        assert_eq!(all, PermissionAnswer::AllowConversation);
    }
}
