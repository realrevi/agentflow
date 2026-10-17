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

/// Why an answer was not delivered (the request stays as it was).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejected {
    /// Never opened, already answered, or its turn ended.
    Unknown,
    /// The id belongs to another conversation.
    WrongConversation,
    /// "Allow for this conversation" is never offered for dangerous commands.
    NotOffered,
}

struct Pending {
    tx: oneshot::Sender<PermissionAnswer>,
    conversation: String,
    dangerous: bool,
}

/// Managed state: permission requests waiting for an answer from the frontend. An answer is
/// delivered at most once, only for its own conversation; a turn that ends (cancel, panel
/// closed, reload) drops its request, which the loop treats as "not run", never as allowed.
#[derive(Clone, Default)]
pub struct PermissionGate {
    waiting: Arc<Mutex<HashMap<u64, Pending>>>,
    next: Arc<AtomicU64>,
}

impl PermissionGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// A new request id and the receiver its answer arrives on.
    pub fn open(
        &self,
        conversation: &str,
        dangerous: bool,
    ) -> (u64, oneshot::Receiver<PermissionAnswer>) {
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        let (tx, rx) = oneshot::channel();
        let pending = Pending {
            tx,
            conversation: conversation.to_owned(),
            dangerous,
        };
        lock(&self.waiting).insert(id, pending);
        (id, rx)
    }

    /// Delivers the user's answer to request `id` of `conversation`.
    pub fn answer(
        &self,
        id: u64,
        conversation: &str,
        answer: PermissionAnswer,
    ) -> Result<(), Rejected> {
        let mut waiting = lock(&self.waiting);
        let result = match waiting.get(&id) {
            None => Err(Rejected::Unknown),
            Some(p) if p.conversation != conversation => Err(Rejected::WrongConversation),
            Some(p) if p.dangerous && answer == PermissionAnswer::AllowConversation => {
                Err(Rejected::NotOffered)
            }
            Some(_) => Ok(()),
        };
        tracing::debug!(
            request = id,
            conversation,
            decision = ?answer,
            outcome = ?result,
            "permission answer"
        );
        result?;
        match waiting.remove(&id) {
            // The turn may have ended a moment ago: then nobody receives it.
            Some(pending) => pending.tx.send(answer).map_err(|_| Rejected::Unknown),
            None => Err(Rejected::Unknown),
        }
    }

    /// Forgets a request whose turn ended without an answer.
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
        let (id, rx) = gate.open("c1", false);
        assert_eq!(gate.answer(id, "c1", PermissionAnswer::AllowOnce), Ok(()));
        assert_eq!(
            gate.answer(id, "c1", PermissionAnswer::AllowOnce),
            Err(Rejected::Unknown),
            "already answered"
        );
        let answer = tauri::async_runtime::block_on(rx).expect("answer");
        assert_eq!(answer, PermissionAnswer::AllowOnce);
        assert_eq!(
            gate.answer(999, "c1", PermissionAnswer::AllowOnce),
            Err(Rejected::Unknown),
            "never opened"
        );
        let (closed, _rx) = gate.open("c1", false);
        gate.close(closed);
        assert_eq!(
            gate.answer(closed, "c1", PermissionAnswer::AllowOnce),
            Err(Rejected::Unknown),
            "turn ended"
        );
    }

    #[test]
    fn answers_only_count_for_their_own_conversation() {
        let gate = PermissionGate::new();
        let (id, mut rx) = gate.open("c1", false);
        assert_eq!(
            gate.answer(id, "c2", PermissionAnswer::AllowConversation),
            Err(Rejected::WrongConversation)
        );
        assert!(rx.try_recv().is_err(), "still pending");
        assert_eq!(gate.answer(id, "c1", PermissionAnswer::AllowOnce), Ok(()));
    }

    #[test]
    fn dangerous_requests_never_take_a_conversation_grant() {
        let gate = PermissionGate::new();
        let (id, mut rx) = gate.open("c1", true);
        assert_eq!(
            gate.answer(id, "c1", PermissionAnswer::AllowConversation),
            Err(Rejected::NotOffered)
        );
        assert!(rx.try_recv().is_err(), "still pending");
        let deny = PermissionAnswer::Deny { reason: None };
        assert_eq!(gate.answer(id, "c1", deny.clone()), Ok(()));
        assert_eq!(tauri::async_runtime::block_on(rx).expect("answer"), deny);
    }

    #[test]
    fn a_dropped_turn_never_reads_as_allowed() {
        let gate = PermissionGate::new();
        let (id, rx) = gate.open("c1", false);
        gate.close(id);
        assert!(tauri::async_runtime::block_on(rx).is_err());
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
