//! Permission answers end to end: a turn that ends while a request waits never runs the
//! call, grants stay in their conversation, dangerous commands never take a grant.

use serde_json::json;

use super::agent::{agent_setup, permission_count, text_reply, tool_call};
use super::*;
use crate::agent::{PermissionAnswer, Rejected};

#[test]
fn cancel_while_waiting_never_runs_the_call() {
    let server = MockServer::start(vec![tool_call(
        "c1",
        "write_file",
        json!({ "path": "a.txt", "content": "no" }),
    )]);
    let setup = agent_setup(&server.base);
    let (request_id, rx) = setup.send("write");
    let mut pending = None;
    let mut events = Vec::new();
    loop {
        let event = rx.recv_timeout(Duration::from_secs(20)).expect("event");
        if let ChatEvent::Permission { request } = &event {
            pending = Some(request.id);
            assert!(setup.base.manager.cancel(request_id));
        }
        let last = matches!(event, ChatEvent::Stop { .. } | ChatEvent::Error { .. });
        events.push(event);
        if last {
            break;
        }
    }
    assert!(matches!(events.last(), Some(ChatEvent::Stop { reason, .. }) if reason == "cancelled"));
    assert!(!setup.file("a.txt").exists());
    // A late click finds nothing to answer.
    let id = pending.expect("asked");
    assert_eq!(
        setup
            .base
            .agent
            .gate
            .answer(id, &setup.conversation, PermissionAnswer::AllowOnce),
        Err(Rejected::Unknown)
    );
    let stored = setup
        .base
        .store
        .get(Some(&setup.ws), &setup.conversation)
        .expect("get");
    let last = stored.messages.last().expect("message");
    assert_eq!(last.role, crate::conversations::Role::Tool);
    assert!(matches!(&last.parts[0],
        crate::providers::Part::ToolResult(r) if r.is_error && r.content.starts_with("Not run")));
}

#[test]
fn dangerous_commands_never_take_a_conversation_grant() {
    let server = MockServer::start(vec![
        tool_call("c1", "run_command", json!({ "command": "rm -rf build" })),
        text_reply("ok"),
    ]);
    let setup = agent_setup(&server.base);
    let (_, rx) = setup.send("clean");
    let mut events = Vec::new();
    loop {
        let event = rx.recv_timeout(Duration::from_secs(20)).expect("event");
        if let ChatEvent::Permission { request } = &event {
            assert!(request.dangerous.is_some());
            let gate = &setup.base.agent.gate;
            assert_eq!(
                gate.answer(
                    request.id,
                    &setup.conversation,
                    PermissionAnswer::AllowConversation
                ),
                Err(Rejected::NotOffered)
            );
            let deny = PermissionAnswer::Deny { reason: None };
            assert_eq!(gate.answer(request.id, &setup.conversation, deny), Ok(()));
        }
        let last = matches!(event, ChatEvent::Stop { .. } | ChatEvent::Error { .. });
        events.push(event);
        if last {
            break;
        }
    }
    assert_eq!(permission_count(&events), 1);
    let stored = setup
        .base
        .store
        .get(Some(&setup.ws), &setup.conversation)
        .expect("get");
    assert!(stored.agent.allowed_tools.is_empty());
}

#[test]
fn grants_and_answers_stay_in_their_conversation() {
    let server = MockServer::start(vec![
        tool_call(
            "c1",
            "write_file",
            json!({ "path": "b.txt", "content": "b" }),
        ),
        text_reply("ok"),
    ]);
    let mut setup = agent_setup(&server.base);
    let first = setup.conversation.clone();
    setup
        .base
        .store
        .update_agent(Some(&setup.ws), &first, |agent| {
            agent.allowed_tools = vec!["write_file".into()];
        })
        .expect("grant");
    setup.conversation = setup
        .base
        .store
        .create(Some(&setup.ws), Some("p1".into()), "model-x", "")
        .expect("create")
        .id;
    let (_, rx) = setup.send("write b");
    let mut events = Vec::new();
    loop {
        let event = rx.recv_timeout(Duration::from_secs(20)).expect("event");
        if let ChatEvent::Permission { request } = &event {
            let gate = &setup.base.agent.gate;
            assert_eq!(
                gate.answer(request.id, &first, PermissionAnswer::AllowOnce),
                Err(Rejected::WrongConversation)
            );
            assert_eq!(
                gate.answer(request.id, &setup.conversation, PermissionAnswer::AllowOnce),
                Ok(())
            );
        }
        let last = matches!(event, ChatEvent::Stop { .. } | ChatEvent::Error { .. });
        events.push(event);
        if last {
            break;
        }
    }
    assert_eq!(
        permission_count(&events),
        1,
        "the other chat's grant does not apply"
    );
    assert!(setup.file("b.txt").exists());
}
