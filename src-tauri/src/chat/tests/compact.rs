//! Compact: older turns become a summary, the last N stay verbatim, the call is in the ledger.

use super::*;
use crate::chat::compact::{older_messages, CompactRequest};
use crate::chat::request::build_request;
use crate::conversations::{ChatMessage, Role};

fn reply(id: &str, text: &str) -> ChatMessage {
    ChatMessage {
        id: id.into(),
        role: Role::Assistant,
        content: text.into(),
        attachments: Vec::new(),
        at: crate::util::now_ms(),
        status: MessageStatus::Complete,
        stop_reason: None,
        error: None,
        usage: None,
        parts: Vec::new(),
        prefix: None,
    }
}

fn three_turns(setup: &Setup) {
    for (n, (question, answer)) in [("q1", "a1"), ("q2", "a2"), ("q3", "a3")]
        .iter()
        .enumerate()
    {
        setup
            .store
            .append_user(WS, &setup.conversation, question, Vec::new())
            .expect("user");
        setup
            .store
            .append_message(WS, &setup.conversation, reply(&format!("r{n}"), answer))
            .expect("reply");
    }
}

#[test]
fn compact_keeps_the_last_turns_and_records_the_summary_call() {
    let server = MockServer::start(vec![Reply::sse(include_str!(
        "../../providers/fixtures/anthropic_basic.sse"
    ))]);
    let mut setup = setup(&server.base, "anthropic", "anthropic");
    setup.settings.chat.compact_keep_turns = 2;
    three_turns(&setup);
    let deps = setup.deps();
    let request = CompactRequest {
        workspace: WS.map(str::to_owned),
        conversation_id: setup.conversation.clone(),
        panel: Some("chat-1".into()),
    };
    let result =
        tauri::async_runtime::block_on(super::super::compact(deps, request)).expect("compact");
    assert_eq!(result.summarized, 2, "q1 and its reply");
    assert_eq!(result.compaction.through, "r0");
    assert_eq!(result.compaction.summary, "Hello!");

    let sent = server.requests()[0].json();
    let prompt = sent["messages"][0]["content"]
        .as_str()
        .expect("prompt")
        .to_owned();
    assert!(prompt.contains("User:\nq1") && prompt.contains("Assistant:\na1"));
    assert!(!prompt.contains("q2"), "kept turns are not summarized");

    let stored = setup.store.get(WS, &setup.conversation).expect("get");
    assert_eq!(
        stored.messages.len(),
        6,
        "the chat still shows every message"
    );
    assert_eq!(stored.compaction.as_ref(), Some(&result.compaction));
    let (next, _) = setup
        .store
        .append_user(WS, &setup.conversation, "q4", Vec::new())
        .expect("user");
    let chat = build_request(&next, &setup.settings.providers[0], false).expect("request");
    let first = chat.turns[0].content();
    assert!(first.starts_with("Summary of the earlier part of this conversation:\n\nHello!"));
    assert!(first.ends_with("q2") && !first.contains("q1"));
    assert_eq!(chat.turns.len(), 5, "summary+q2, a2, q3, a3, q4");

    let summary = setup.tracker.summary(0);
    assert_eq!(summary.compact.requests, 1);
    assert_eq!(summary.compact.tokens.output_tokens, 15);
}

#[test]
fn nothing_to_compact_while_only_kept_turns_exist() {
    let setup = setup("https://api.anthropic.com", "anthropic", "anthropic");
    three_turns(&setup);
    let conversation = setup.store.get(WS, &setup.conversation).expect("get");
    assert_eq!(older_messages(&conversation, 1).expect("older").len(), 4);
    assert!(older_messages(&conversation, 3).is_err());
    assert!(older_messages(&conversation, 10).is_err());
}
