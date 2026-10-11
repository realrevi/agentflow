use super::checks::{auto_title, is_valid_id};
use super::*;
use crate::error::AppError;
use crate::providers::TokenUsage;
use crate::util::test_dir::TestDir;

const WS: Option<&str> = Some("/home/me/projects/demo-app");

fn store(dir: &TestDir) -> ConversationStore {
    ConversationStore::new(dir.path().join("conversations"))
}

fn reply(text: &str, status: MessageStatus) -> ChatMessage {
    ChatMessage {
        id: "m-reply".into(),
        role: Role::Assistant,
        content: text.into(),
        attachments: Vec::new(),
        at: 1,
        status,
        stop_reason: Some("end_turn".into()),
        error: None,
        usage: Some(UsageRecord {
            provider_id: "p1".into(),
            preset: "anthropic".into(),
            model: "m".into(),
            at: 2,
            tokens: TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
                cache_read_tokens: 3,
                cache_write_tokens: 1,
            },
        }),
    }
}

#[test]
fn conversation_file_round_trip() {
    let dir = TestDir::new("conversations");
    let store = store(&dir);
    let created = store
        .create(WS, Some("p1".into()), "claude-x", "Be brief.")
        .expect("create");
    let attachment = Attachment {
        path: "src/main.rs".into(),
        content: "fn main() {}\n".into(),
    };
    let (_, user) = store
        .append_user(WS, &created.id, "Explain this file", vec![attachment])
        .expect("user");
    store
        .append_assistant(
            WS,
            &created.id,
            reply("It prints nothing.", MessageStatus::Complete),
        )
        .expect("assistant");

    // A fresh store (an app restart) reads exactly what was written.
    let loaded = ConversationStore::new(dir.path().join("conversations"))
        .get(WS, &created.id)
        .expect("get");
    assert_eq!(loaded.title, "Explain this file");
    assert_eq!(loaded.system_prompt, "Be brief.");
    assert_eq!(loaded.workspace.as_deref(), WS);
    assert_eq!(loaded.messages.len(), 2);
    assert_eq!(loaded.messages[0], user);
    let usage = loaded.messages[1].usage.as_ref().expect("usage");
    assert_eq!(usage.tokens.cache_read_tokens, 3);
    assert_eq!(usage.model, "m");

    // The JSON uses camelCase and flattens the token counts into the usage record.
    let raw = std::fs::read_to_string(
        dir.path()
            .join("conversations")
            .join(crate::util::workspace_key(WS))
            .join(format!("{}.json", created.id)),
    )
    .expect("read");
    assert!(raw.contains("\"cacheWriteTokens\": 1") && raw.contains("\"providerId\": \"p1\""));
}

#[test]
fn lists_newest_first_per_workspace() {
    let dir = TestDir::new("conversations");
    let store = store(&dir);
    let first = store.create(WS, None, "", "").expect("create");
    let second = store.create(WS, None, "", "").expect("create");
    store.create(None, None, "", "").expect("other workspace");
    std::thread::sleep(std::time::Duration::from_millis(5));
    store
        .append_user(WS, &first.id, "hi", Vec::new())
        .expect("user");
    let list = store.list(WS).expect("list");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, first.id);
    assert_eq!(list[0].message_count, 1);
    assert_eq!(list[1].id, second.id);
    assert_eq!(store.list(None).expect("list").len(), 1);
    assert!(store.list(Some("/never/used")).expect("list").is_empty());
}

#[test]
fn rename_update_and_delete_to_trash() {
    let dir = TestDir::new("conversations");
    let store = store(&dir);
    let created = store
        .create(WS, Some("p1".into()), "m1", "")
        .expect("create");
    let patch: ConversationPatch =
        serde_json::from_str(r#"{ "title": "  My   chat ", "model": "m2" }"#).expect("patch");
    let summary = store.update(WS, &created.id, patch).expect("update");
    assert_eq!(summary.title, "My chat");
    assert_eq!(summary.model, "m2");
    assert_eq!(summary.provider_id.as_deref(), Some("p1"), "untouched");

    let clear: ConversationPatch = serde_json::from_str(r#"{ "providerId": null }"#).expect("p");
    assert_eq!(
        store.update(WS, &created.id, clear).expect("u").provider_id,
        None
    );
    let bad: ConversationPatch = serde_json::from_str(r#"{ "providerId": "../x" }"#).expect("p");
    assert!(store.update(WS, &created.id, bad).is_err());

    store.delete(WS, &created.id).expect("delete");
    assert!(store.list(WS).expect("list").is_empty());
    assert!(matches!(
        store.get(WS, &created.id),
        Err(AppError::NotFound(_))
    ));
    let trash = dir.path().join("conversations/.trash");
    assert_eq!(std::fs::read_dir(trash).expect("trash").count(), 1);
    assert!(store.delete(WS, &created.id).is_err());
}

#[test]
fn retry_drops_failed_replies_only() {
    let dir = TestDir::new("conversations");
    let store = store(&dir);
    let id = store.create(WS, None, "", "").expect("create").id;
    assert!(store.prepare_retry(WS, &id).is_err(), "nothing sent yet");
    store.append_user(WS, &id, "one", Vec::new()).expect("user");
    store
        .append_assistant(WS, &id, reply("partial", MessageStatus::Error))
        .expect("assistant");
    let ready = store.prepare_retry(WS, &id).expect("retry");
    assert_eq!(ready.messages.len(), 1);
    store
        .append_assistant(WS, &id, reply("done", MessageStatus::Complete))
        .expect("assistant");
    assert!(
        store.prepare_retry(WS, &id).is_err(),
        "a finished reply is not retried"
    );
}

#[test]
fn limits_and_bad_input() {
    let dir = TestDir::new("conversations");
    let store = store(&dir);
    let id = store.create(WS, None, "", "").expect("create").id;
    let big = Attachment {
        path: "big.txt".into(),
        content: "x".repeat(MAX_ATTACHMENT_BYTES + 1),
    };
    assert!(matches!(
        store.append_user(WS, &id, "look", vec![big]),
        Err(AppError::TooLarge { .. })
    ));
    assert!(store.append_user(WS, &id, "   ", Vec::new()).is_err());
    assert!(store.get(WS, "../../etc/passwd").is_err());
    assert!(store.create(WS, Some("bad id".into()), "", "").is_err());
    assert!(store.create(WS, None, "line\nbreak", "").is_err());
    assert!(is_valid_id(
        &store.create(WS, None, "", "").expect("create").id
    ));
}

#[test]
fn titles_come_from_the_first_prompt() {
    assert_eq!(auto_title("  fix   the\nbuild  ", &[]), "fix the build");
    let long = "word ".repeat(30);
    let title = auto_title(&long, &[]);
    assert_eq!(title.chars().count(), 60);
    assert!(title.ends_with('…'));
    let file = Attachment {
        path: "src/lib.rs".into(),
        content: String::new(),
    };
    assert_eq!(auto_title("", &[file]), "lib.rs");
}

#[test]
fn unreadable_files_are_skipped_in_the_list() {
    let dir = TestDir::new("conversations");
    let store = store(&dir);
    store.create(WS, None, "", "").expect("create");
    let folder = dir
        .path()
        .join("conversations")
        .join(crate::util::workspace_key(WS));
    std::fs::write(folder.join("broken.json"), "{oops").expect("write");
    assert_eq!(store.list(WS).expect("list").len(), 1);
}
