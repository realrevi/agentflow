use super::*;
use crate::conversations::Attachment;
use crate::conversations::Compaction;

fn message(role: Role, content: &str, status: MessageStatus) -> ChatMessage {
    ChatMessage {
        id: format!("m-{content}"),
        role,
        content: content.into(),
        attachments: Vec::new(),
        at: 0,
        status,
        stop_reason: None,
        error: None,
        usage: None,
        parts: Vec::new(),
        prefix: None,
    }
}

fn with_files(mut message: ChatMessage, files: &[(&str, &str)]) -> ChatMessage {
    message.attachments = files
        .iter()
        .map(|(path, content)| Attachment {
            path: (*path).into(),
            content: (*content).into(),
        })
        .collect();
    message
}

fn conversation(messages: Vec<ChatMessage>) -> Conversation {
    Conversation {
        model: "m1".into(),
        system_prompt: "  Be brief.  ".into(),
        messages,
        ..Conversation::default()
    }
}

fn texts(request: &ChatRequest) -> Vec<(ChatRole, String)> {
    request
        .turns
        .iter()
        .map(|t| (t.role, t.content()))
        .collect()
}

#[test]
fn alternates_and_drops_failed_replies() {
    use MessageStatus::*;
    let request = build_request(
        &conversation(vec![
            message(Role::Assistant, "orphan", Complete),
            message(Role::User, "one", Complete),
            message(Role::Assistant, "partial", Error),
            message(Role::User, "two", Complete),
            message(Role::Assistant, "half", Cancelled),
            message(Role::Assistant, "", Cancelled),
            message(Role::User, "three", Complete),
        ]),
        &ProviderEntry::default(),
        false,
    )
    .expect("request");
    assert_eq!(
        texts(&request),
        vec![
            (ChatRole::User, "one\n\ntwo".to_owned()),
            (ChatRole::Assistant, "half".to_owned()),
            (ChatRole::User, "three".to_owned()),
        ]
    );
    assert_eq!(request.system, "Be brief.");
    assert_eq!(request.max_tokens, ProviderEntry::default().max_tokens);
    assert!(!request.caching);
    assert!(request
        .turns
        .iter()
        .all(|t| t.blocks.iter().all(|b| !b.cache)));
}

#[test]
fn needs_a_user_message_last() {
    let done = conversation(vec![
        message(Role::User, "q", MessageStatus::Complete),
        message(Role::Assistant, "a", MessageStatus::Complete),
    ]);
    let entry = ProviderEntry::default();
    assert!(build_request(&done, &entry, false).is_err());
    assert!(build_request(&conversation(Vec::new()), &entry, false).is_err());
}

#[test]
fn attachments_are_fenced_with_their_path() {
    let files = vec![
        Attachment {
            path: "src/main.rs".into(),
            content: "fn main() {}".into(),
        },
        Attachment {
            path: "docs/README".into(),
            content: "use ```code``` here\n".into(),
        },
    ];
    let content = user_content("Explain", &files);
    assert_eq!(
        content,
        "File: src/main.rs\n```rs\nfn main() {}\n```\n\n\
         File: docs/README\n````\nuse ```code``` here\n````\n\nExplain"
    );
}

#[test]
fn a_file_attached_again_is_sent_once_in_its_latest_version() {
    use MessageStatus::Complete;
    let request = build_request(
        &conversation(vec![
            with_files(
                message(Role::User, "look", Complete),
                &[("a.rs", "v1"), ("b.rs", "b")],
            ),
            message(Role::Assistant, "ok", Complete),
            with_files(message(Role::User, "again", Complete), &[("a.rs", "v2")]),
        ]),
        &ProviderEntry::default(),
        true,
    )
    .expect("request");
    let first = request.turns[0].content();
    assert!(first.starts_with("File: a.rs (a newer copy is attached later"));
    assert!(!first.contains("v1"), "the old copy is not sent");
    assert!(first.contains("```rs\nb\n```"), "other files stay");
    let last = request.turns[2].content();
    assert!(last.contains("File: a.rs\n```rs\nv2\n```") && last.ends_with("again"));
    // Breakpoints: end of request, stable prefix, newest attachment.
    let marked: Vec<(usize, usize)> = request
        .turns
        .iter()
        .enumerate()
        .flat_map(|(t, turn)| {
            turn.blocks
                .iter()
                .enumerate()
                .filter(|(_, b)| b.cache)
                .map(move |(b, _)| (t, b))
        })
        .collect();
    assert_eq!(marked, vec![(1, 0), (2, 0), (2, 1)]);
}

#[test]
fn a_compaction_replaces_the_messages_it_covers() {
    use MessageStatus::Complete;
    let mut chat = conversation(vec![
        message(Role::User, "old question", Complete),
        message(Role::Assistant, "old answer", Complete),
        message(Role::User, "new question", Complete),
    ]);
    chat.compaction = Some(Compaction {
        summary: "They talked about X.".into(),
        through: "m-old answer".into(),
        at: 1,
        provider_id: "p1".into(),
        model: "m1".into(),
    });
    let request = build_request(&chat, &ProviderEntry::default(), false).expect("request");
    assert_eq!(request.turns.len(), 1);
    let text = request.turns[0].content();
    assert!(text.starts_with("Summary of the earlier part of this conversation:\n\nThey talked"));
    assert!(text.ends_with("new question") && !text.contains("old"));
}

mod agent_parts {
    use super::*;
    use crate::providers::{Reasoning, ToolCall, ToolResult};
    use serde_json::json;

    fn step(role: Role, parts: Vec<Part>, prefix: Option<&str>, at: u64) -> ChatMessage {
        let mut message = message(role, "", MessageStatus::Complete);
        message.parts = parts;
        message.prefix = prefix.map(str::to_owned);
        message.at = at;
        message
    }

    fn call(id: &str) -> Part {
        Part::ToolCall(ToolCall {
            id: id.into(),
            name: "list_dir".into(),
            input: json!({}),
        })
    }

    fn thinking() -> Part {
        Part::Reasoning(Reasoning {
            text: "t".into(),
            signature: Some("s".into()),
            ..Reasoning::default()
        })
    }

    fn anthropic(prefix: &str) -> RequestOptions {
        RequestOptions {
            anthropic: true,
            prefix: Some(prefix.into()),
            ..RequestOptions::default()
        }
    }

    #[test]
    fn tool_steps_become_turns_and_dangling_calls_get_a_result() {
        use MessageStatus::Complete;
        let chat = conversation(vec![
            message(Role::User, "list", Complete),
            step(
                Role::Assistant,
                vec![thinking(), call("c1"), call("c2")],
                Some("p"),
                2,
            ),
            step(
                Role::Tool,
                vec![Part::ToolResult(ToolResult {
                    call_id: "c1".into(),
                    content: "a/".into(),
                    is_error: false,
                    ..ToolResult::default()
                })],
                None,
                3,
            ),
            message(Role::User, "and now?", Complete),
        ]);
        let request =
            build_with(&chat, &ProviderEntry::default(), &anthropic("p")).expect("request");
        assert_eq!(request.turns.len(), 3);
        let results: Vec<(String, bool)> = request.turns[2]
            .blocks
            .iter()
            .filter_map(|b| match &b.part {
                Part::ToolResult(r) => Some((r.call_id.clone(), r.is_error)),
                _ => None,
            })
            .collect();
        assert_eq!(results, [("c2".to_owned(), true), ("c1".to_owned(), false)]);
        assert_eq!(request.turns[2].content(), "and now?");
        assert!(matches!(
            request.turns[1].blocks[0].part,
            Part::Reasoning(_)
        ));
    }

    #[test]
    fn anthropic_thinking_is_replayed_only_under_the_same_prefix() {
        use MessageStatus::Complete;
        let chat = conversation(vec![
            message(Role::User, "q", Complete),
            step(
                Role::Assistant,
                vec![thinking(), Part::Text { text: "a".into() }],
                Some("old"),
                2,
            ),
            message(Role::User, "q2", Complete),
        ]);
        let entry = ProviderEntry::default();
        let has_thinking = |options: &RequestOptions| {
            build_with(&chat, &entry, options).expect("request").turns[1]
                .blocks
                .iter()
                .any(|b| matches!(b.part, Part::Reasoning(_)))
        };
        assert!(has_thinking(&anthropic("old")));
        assert!(
            !has_thinking(&anthropic("new")),
            "the system prompt or tools changed"
        );
        let openai = RequestOptions {
            prefix: Some("new".into()),
            ..RequestOptions::default()
        };
        assert!(
            has_thinking(&openai),
            "OpenAI-compatible reasoning always goes back"
        );
    }
}
