//! Agent turns end to end against the mock server: permissions (allow once, allow for the
//! conversation, deny with a reason, read-only), the iteration limit and cancel mid-tool.

use std::time::Instant;

use serde_json::{json, Value};

use super::*;
use crate::agent::PermissionAnswer;
use crate::conversations::{ConversationPatch, PermissionMode};

pub(super) struct AgentSetup {
    pub(super) base: Setup,
    pub(super) ws: String,
    pub(super) conversation: String,
    _work: TestDir,
}

pub(super) fn agent_setup(server: &str) -> AgentSetup {
    let base = setup(server, "openai", "custom");
    let work = TestDir::new("agent-ws");
    std::fs::write(work.path().join("README.md"), "# demo\n").expect("write");
    let ws = work.path().to_string_lossy().into_owned();
    let conversation = base
        .store
        .create(Some(&ws), Some("p1".into()), "model-x", "")
        .expect("create")
        .id;
    AgentSetup {
        base,
        ws,
        conversation,
        _work: work,
    }
}

impl AgentSetup {
    pub(super) fn send(&self, text: &str) -> (u64, mpsc::Receiver<ChatEvent>) {
        let (tx, rx) = mpsc::channel();
        let request = ChatSendRequest {
            workspace: Some(self.ws.clone()),
            conversation_id: self.conversation.clone(),
            text: Some(text.into()),
            attachments: Vec::new(),
            panel: Some("chat-1".into()),
            allow_once: None,
        };
        let started =
            tauri::async_runtime::block_on(start_chat(self.base.deps(), request, move |event| {
                let _ = tx.send(event);
            }))
            .expect("started");
        (started.request_id, rx)
    }

    pub(super) fn mode(&self, mode: PermissionMode) {
        let patch = ConversationPatch {
            permission_mode: Some(mode),
            ..ConversationPatch::default()
        };
        self.base
            .store
            .update(Some(&self.ws), &self.conversation, patch)
            .expect("mode");
    }

    pub(super) fn file(&self, name: &str) -> std::path::PathBuf {
        std::path::Path::new(&self.ws).join(name)
    }
}

/// One OpenAI-style streamed tool call.
pub(super) fn tool_call(id: &str, name: &str, args: Value) -> Reply {
    let chunk = json!({ "choices": [{ "index": 0, "delta": { "tool_calls": [{
        "index": 0, "id": id, "type": "function",
        "function": { "name": name, "arguments": args.to_string() } }] },
        "finish_reason": null }] });
    let finish = json!({ "choices": [{ "index": 0, "delta": {}, "finish_reason": "tool_calls" }],
        "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 } });
    Reply::sse(&format!(
        "data: {chunk}\n\ndata: {finish}\n\ndata: [DONE]\n\n"
    ))
}

pub(super) fn text_reply(text: &str) -> Reply {
    let chunk = json!({ "choices": [{ "index": 0, "delta": { "content": text }, "finish_reason": "stop" }],
        "usage": { "prompt_tokens": 12, "completion_tokens": 3, "total_tokens": 15 } });
    Reply::sse(&format!("data: {chunk}\n\ndata: [DONE]\n\n"))
}

/// Events until the end, answering permission requests with `answer`.
pub(super) fn run_answering(
    setup: &AgentSetup,
    rx: &mpsc::Receiver<ChatEvent>,
    answer: impl Fn() -> PermissionAnswer,
) -> Vec<ChatEvent> {
    let mut events = Vec::new();
    loop {
        let event = rx.recv_timeout(Duration::from_secs(20)).expect("event");
        if let ChatEvent::Permission { request } = &event {
            assert!(setup
                .base
                .agent
                .gate
                .answer(request.id, &setup.conversation, answer())
                .is_ok());
        }
        let last = matches!(event, ChatEvent::Stop { .. } | ChatEvent::Error { .. });
        events.push(event);
        if last {
            return events;
        }
    }
}

pub(super) fn tool_results(server: &MockServer, request: usize) -> Vec<Value> {
    let body = server.requests()[request].json();
    body["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .filter(|m| m["role"] == "tool")
        .cloned()
        .collect()
}

pub(super) fn permission_count(events: &[ChatEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, ChatEvent::Permission { .. }))
        .count()
}

#[test]
fn allow_once_runs_the_write_and_the_model_gets_the_result() {
    let server = MockServer::start(vec![
        tool_call(
            "call_1",
            "write_file",
            json!({ "path": "a.txt", "content": "hi\n" }),
        ),
        text_reply("Done."),
    ]);
    let setup = agent_setup(&server.base);
    let (_, rx) = setup.send("make a.txt");
    let events = run_answering(&setup, &rx, || PermissionAnswer::AllowOnce);
    assert_eq!(permission_count(&events), 1);
    let Some(ChatEvent::Permission { request }) = events
        .iter()
        .find(|e| matches!(e, ChatEvent::Permission { .. }))
    else {
        panic!("no permission");
    };
    let preview = request.preview.as_ref().expect("preview");
    assert_eq!(
        (preview.before.as_deref(), preview.after.as_str()),
        (None, "hi\n")
    );
    assert_eq!(
        std::fs::read_to_string(setup.file("a.txt")).expect("read"),
        "hi\n"
    );
    let results = tool_results(&server, 1);
    assert_eq!(results[0]["tool_call_id"], "call_1");
    assert!(results[0]["content"]
        .as_str()
        .unwrap_or("")
        .starts_with("Created a.txt"));
    // The first request offered the tools and the default system prompt.
    let first = server.requests()[0].json();
    assert!(first["tools"].as_array().is_some_and(|t| t.len() == 7));
    let system = first["messages"][0]["content"].as_str().unwrap_or("");
    assert!(system.starts_with("You are an AI agent running inside AgentFlow"));
    assert!(matches!(events.last(), Some(ChatEvent::Stop { reason, .. }) if reason == "stop"));
    // Each model call's usage is in the ledger.
    assert_eq!(setup.base.tracker.panel_totals("chat-1", 0).requests, 2);
}

#[test]
fn allow_for_the_conversation_stops_asking_for_that_tool() {
    let server = MockServer::start(vec![
        tool_call(
            "c1",
            "write_file",
            json!({ "path": "a.txt", "content": "1" }),
        ),
        text_reply("ok"),
        tool_call(
            "c2",
            "write_file",
            json!({ "path": "b.txt", "content": "2" }),
        ),
        text_reply("ok"),
    ]);
    let setup = agent_setup(&server.base);
    let (_, rx) = setup.send("first");
    let first = run_answering(&setup, &rx, || PermissionAnswer::AllowConversation);
    assert_eq!(permission_count(&first), 1);
    let (_, rx) = setup.send("second");
    let second = run_answering(&setup, &rx, || panic!("asked again"));
    assert_eq!(permission_count(&second), 0);
    assert!(setup.file("b.txt").exists());
    let stored = setup
        .base
        .store
        .get(Some(&setup.ws), &setup.conversation)
        .expect("get");
    assert_eq!(stored.agent.allowed_tools, ["write_file"]);
}

#[test]
fn deny_sends_the_reason_to_the_model() {
    let server = MockServer::start(vec![
        tool_call("c1", "run_command", json!({ "command": "npm install" })),
        text_reply("Understood."),
    ]);
    let setup = agent_setup(&server.base);
    let (_, rx) = setup.send("install");
    let events = run_answering(&setup, &rx, || PermissionAnswer::Deny {
        reason: Some("use pnpm".into()),
    });
    // The denial shows at once: ToolEnd right after the answer, before the tool message.
    let end = events
        .iter()
        .position(|e| matches!(e, ChatEvent::ToolEnd { result }
            if result.outcome == crate::providers::ToolOutcome::Denied && result.duration_ms.is_none()))
        .expect("denied ToolEnd");
    let stored = events
        .iter()
        .position(|e| matches!(e, ChatEvent::Message { message } if message.role == crate::conversations::Role::Tool))
        .expect("tool message");
    assert!(end < stored);
    assert!(!events
        .iter()
        .any(|e| matches!(e, ChatEvent::ToolStart { .. })));
    let results = tool_results(&server, 1);
    assert_eq!(
        results[0]["content"],
        "Error: The user denied this tool call. Their reason: use pnpm"
    );
}

#[test]
fn read_only_offers_no_write_or_command_tools() {
    let server = MockServer::start(vec![
        tool_call(
            "c1",
            "write_file",
            json!({ "path": "x.txt", "content": "no" }),
        ),
        text_reply("Sorry."),
    ]);
    let setup = agent_setup(&server.base);
    setup.mode(PermissionMode::ReadOnly);
    let (_, rx) = setup.send("try");
    let events = run_answering(&setup, &rx, || panic!("read-only never asks"));
    assert_eq!(permission_count(&events), 0);
    let first = server.requests()[0].json();
    let names: Vec<&str> = first["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|t| t["function"]["name"].as_str())
        .collect();
    assert_eq!(names, ["list_dir", "read_file", "glob", "search"]);
    assert!(!setup.file("x.txt").exists());
    assert!(tool_results(&server, 1)[0]["content"]
        .as_str()
        .unwrap_or("")
        .contains("read-only"));
}

#[test]
fn the_iteration_limit_ends_the_turn() {
    let server = MockServer::start(vec![
        tool_call("c1", "list_dir", json!({})),
        tool_call("c2", "list_dir", json!({})),
        tool_call("c3", "list_dir", json!({})),
    ]);
    let mut setup = agent_setup(&server.base);
    setup.base.settings.agent.max_iterations = 2;
    let (_, rx) = setup.send("loop");
    let events = run_answering(&setup, &rx, || panic!("reads never ask"));
    assert!(events
        .iter()
        .any(|e| matches!(e, ChatEvent::Notice { kind, .. } if kind == "maxIterations")));
    assert!(
        matches!(events.last(), Some(ChatEvent::Stop { reason, .. }) if reason == "max_iterations")
    );
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn cancel_stops_a_running_command_and_the_turn() {
    let sleep = if cfg!(windows) {
        "Start-Sleep -Seconds 30"
    } else {
        "sleep 30"
    };
    let server = MockServer::start(vec![tool_call(
        "c1",
        "run_command",
        json!({ "command": sleep }),
    )]);
    let setup = agent_setup(&server.base);
    setup
        .base
        .store
        .update_agent(Some(&setup.ws), &setup.conversation, |agent| {
            agent.allowed_tools = vec!["run_command".into()];
        })
        .expect("allow");
    let (request_id, rx) = setup.send("wait");
    let started = Instant::now();
    let mut events = Vec::new();
    loop {
        let event = rx.recv_timeout(Duration::from_secs(20)).expect("event");
        if matches!(event, ChatEvent::ToolStart { .. }) {
            std::thread::sleep(Duration::from_millis(300));
            assert!(setup.base.manager.cancel(request_id));
        }
        let last = matches!(event, ChatEvent::Stop { .. } | ChatEvent::Error { .. });
        events.push(event);
        if last {
            break;
        }
    }
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "killed promptly"
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, ChatEvent::ToolEnd { result } if result.is_error && result.duration_ms.is_some())));
    assert!(matches!(events.last(), Some(ChatEvent::Stop { reason, .. }) if reason == "cancelled"));
    // The stored history stays valid: the call has a result.
    let stored = setup
        .base
        .store
        .get(Some(&setup.ws), &setup.conversation)
        .expect("get");
    assert_eq!(
        stored.messages.last().map(|m| m.role),
        Some(crate::conversations::Role::Tool)
    );
}
