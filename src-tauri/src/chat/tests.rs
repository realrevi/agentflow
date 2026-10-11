//! A reply end to end: stored conversation -> mock provider stream -> events on the
//! "channel" -> stored assistant message with its usage record.

use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use super::*;
use crate::conversations::{ConversationStore, MessageStatus};
use crate::providers::tests::mock::{MockServer, Reply};
use crate::providers::{ErrorKind, Http, KeyVault, MemoryStore};
use crate::settings::{ProviderEntry, Settings};
use crate::util::test_dir::TestDir;

const KEY: &str = "sk-ant-test-0123456789abcdefghij";
const WS: Option<&str> = Some("/work/demo");

struct Setup {
    _dir: TestDir,
    store: ConversationStore,
    vault: KeyVault,
    manager: ChatManager,
    settings: Settings,
    conversation: String,
}

fn setup(base: &str, kind: &str, preset: &str) -> Setup {
    let dir = TestDir::new("chat");
    let store = ConversationStore::new(dir.path().join("conversations"));
    let vault = KeyVault::new(Arc::new(MemoryStore::default()));
    vault.set("p1", KEY).expect("key");
    let settings = Settings {
        providers: vec![ProviderEntry {
            id: "p1".into(),
            kind: kind.into(),
            preset: preset.into(),
            base_url: base.into(),
            ..ProviderEntry::default()
        }],
        ..Settings::default()
    };
    let conversation = store
        .create(WS, Some("p1".into()), "model-x", "")
        .expect("create")
        .id;
    Setup {
        _dir: dir,
        store,
        vault,
        manager: ChatManager::with_http(Http::for_tests(Duration::from_secs(5))),
        settings,
        conversation,
    }
}

impl Setup {
    fn send(&self, text: Option<&str>) -> (AppResult<ChatStarted>, mpsc::Receiver<ChatEvent>) {
        let (tx, rx) = mpsc::channel();
        let deps = ChatDeps {
            settings: self.settings.clone(),
            store: self.store.clone(),
            vault: self.vault.clone(),
            manager: self.manager.clone(),
        };
        let request = ChatSendRequest {
            workspace: WS.map(str::to_owned),
            conversation_id: self.conversation.clone(),
            text: text.map(str::to_owned),
            attachments: Vec::new(),
        };
        let started = tauri::async_runtime::block_on(start_chat(deps, request, move |event| {
            let _ = tx.send(event);
        }));
        (started, rx)
    }
}

use crate::error::AppResult;

/// All events until the final Stop/Error.
fn collect(rx: &mpsc::Receiver<ChatEvent>) -> Vec<ChatEvent> {
    let mut events = Vec::new();
    loop {
        let event = rx.recv_timeout(Duration::from_secs(10)).expect("event");
        let last = matches!(event, ChatEvent::Stop { .. } | ChatEvent::Error { .. });
        events.push(event);
        if last {
            return events;
        }
    }
}

fn text_of(events: &[ChatEvent]) -> String {
    events
        .iter()
        .filter_map(|e| match e {
            ChatEvent::Delta { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_reply_streams_and_is_stored_with_usage() {
    let server = MockServer::start(vec![Reply::sse(include_str!(
        "../providers/fixtures/anthropic_thinking_cache.sse"
    ))]);
    let setup = setup(&server.base, "anthropic", "anthropic");
    let (started, rx) = setup.send(Some("Hello there"));
    let started = started.expect("started");
    assert_eq!(started.user_message.expect("user").content, "Hello there");
    let events = collect(&rx);
    assert_eq!(text_of(&events), "Hi there! How can I help?");
    assert!(events.iter().any(|e| matches!(e, ChatEvent::Usage { .. })));
    let Some(ChatEvent::Stop { reason, message }) = events.last() else {
        panic!("no stop: {events:?}");
    };
    assert_eq!(reason, "end_turn");
    let message = message.as_ref().expect("stored reply");

    let stored = setup.store.get(WS, &setup.conversation).expect("get");
    assert_eq!(stored.title, "Hello there");
    assert_eq!(stored.messages.len(), 2);
    assert_eq!(&stored.messages[1], message);
    let usage = message.usage.as_ref().expect("usage record");
    assert_eq!(
        (usage.provider_id.as_str(), usage.preset.as_str()),
        ("p1", "anthropic")
    );
    assert_eq!(usage.model, "model-x");
    assert_eq!(usage.tokens.cache_read_tokens, 2048);
    assert_eq!(usage.tokens.output_tokens, 42);
    assert!(usage.at > 0);

    // The provider got the stored history.
    let body = server.requests()[0].json();
    assert_eq!(body["messages"][0]["content"], "Hello there");
}

#[test]
fn stop_keeps_the_partial_reply() {
    let full = include_str!("../providers/fixtures/anthropic_basic.sse");
    let first = &full[..full.find("event: content_block_stop").expect("cut")];
    let server = MockServer::start(vec![Reply::sse(first).hold(Duration::from_secs(10))]);
    let setup = setup(&server.base, "anthropic", "anthropic");
    let (started, rx) = setup.send(Some("Write a long story"));
    let id = started.expect("started").request_id;
    // Wait for the first delta, then press Stop.
    loop {
        if let ChatEvent::Delta { .. } = rx.recv_timeout(Duration::from_secs(5)).expect("delta") {
            break;
        }
    }
    assert!(setup.manager.cancel(id));
    let events = collect(&rx);
    let Some(ChatEvent::Stop { reason, message }) = events.last() else {
        panic!("no stop: {events:?}");
    };
    assert_eq!(reason, "cancelled");
    let message = message.as_ref().expect("partial reply stored");
    assert_eq!(message.status, MessageStatus::Cancelled);
    assert!(message.content.starts_with("Hello"), "{}", message.content);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while setup.manager.cancel(id) && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10)); // the job unregisters right after
    }
    assert!(
        !setup.manager.cancel(id),
        "a finished reply cannot be stopped again"
    );
}

#[test]
fn errors_are_redacted_and_retry_resends() {
    let echo = format!(
        r#"{{"type":"error","error":{{"type":"authentication_error","message":"invalid x-api-key {KEY}"}}}}"#
    );
    let server = MockServer::start(vec![
        Reply::json(401, &echo),
        Reply::sse(include_str!("../providers/fixtures/anthropic_basic.sse")),
    ]);
    let setup = setup(&server.base, "anthropic", "anthropic");
    let (started, rx) = setup.send(Some("Hi"));
    started.expect("started");
    let events = collect(&rx);
    let Some(ChatEvent::Error { error, message }) = events.last() else {
        panic!("no error: {events:?}");
    };
    assert_eq!(error.kind, ErrorKind::Auth);
    assert!(!error.message.contains("0123456789"), "{}", error.message);
    assert!(message.is_none(), "nothing arrived, nothing stored");
    let json = serde_json::to_string(&events).expect("json");
    assert!(!json.contains(KEY), "the key never reaches the frontend");

    let (started, rx) = setup.send(None);
    assert!(started.expect("retry").user_message.is_none());
    let events = collect(&rx);
    assert_eq!(text_of(&events), "Hello!");
    let stored = setup.store.get(WS, &setup.conversation).expect("get");
    assert_eq!(stored.messages.len(), 2, "one user message, one reply");
    let bodies: Vec<_> = server.requests().iter().map(|r| r.json()).collect();
    assert_eq!(bodies[0]["messages"], bodies[1]["messages"]);
}

#[test]
fn setup_problems_are_reported_before_anything_is_stored() {
    let setup = setup("https://api.anthropic.com", "anthropic", "anthropic");
    setup.vault.delete("p1").expect("remove key");
    let (started, _) = setup.send(Some("Hi"));
    let err = started.expect_err("no key");
    assert!(err.to_string().contains("no API key"), "{err}");
    assert!(setup
        .store
        .get(WS, &setup.conversation)
        .expect("get")
        .messages
        .is_empty());

    let mut no_provider = setup.settings.clone();
    no_provider.providers.clear();
    let other = Setup {
        settings: no_provider,
        ..setup
    };
    let (started, _) = other.send(Some("Hi"));
    assert!(started
        .expect_err("no provider")
        .to_string()
        .contains("Choose a provider"));
}

#[test]
fn one_reply_at_a_time_per_chat() {
    let full = include_str!("../providers/fixtures/anthropic_basic.sse");
    let server = MockServer::start(vec![Reply::sse(full).hold(Duration::from_secs(3))]);
    let setup = setup(&server.base, "anthropic", "anthropic");
    let (first, rx) = setup.send(Some("one"));
    let id = first.expect("first").request_id;
    let (second, _) = setup.send(Some("two"));
    assert!(second.is_err(), "busy");
    setup.manager.cancel(id);
    collect(&rx);
}
