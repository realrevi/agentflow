use super::*;
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
    let files = [
        Attachment {
            path: "src/main.rs".into(),
            content: "fn main() {}".into(),
        },
        Attachment {
            path: "docs/README".into(),
            content: "use ```code``` here\n".into(),
        },
    ];
    let content = files.iter().map(fenced).collect::<String>() + "Explain";
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
