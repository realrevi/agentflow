use std::sync::{Arc, Mutex};

use super::*;
use crate::conversations::{self, ChatMessage, Conversation, MessageStatus, Role};
use crate::error::AppError;
use crate::providers::TokenUsage;
use crate::settings::{Budgets, UsageSettings};
use crate::tokens::record::{sample, Source};
use crate::util::test_dir::TestDir;

fn tracker(dir: &TestDir) -> UsageTracker {
    UsageTracker::new(dir.path().join("usage"), CliRoots::default())
}

fn events(tracker: &UsageTracker) -> Arc<Mutex<Vec<UsageEvent>>> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    tracker.subscribe(
        0,
        Box::new(move |event| sink.lock().expect("lock").push(event)),
    );
    seen
}

fn reply(id: &str, input: u64) -> ChatMessage {
    ChatMessage {
        id: id.into(),
        role: Role::Assistant,
        content: "ok".into(),
        attachments: Vec::new(),
        at: now_ms(),
        status: MessageStatus::Complete,
        stop_reason: None,
        error: None,
        usage: Some(conversations::UsageRecord {
            provider_id: "p1".into(),
            preset: "anthropic".into(),
            model: "claude-sonnet-4-6".into(),
            at: now_ms(),
            tokens: TokenUsage {
                input_tokens: input,
                output_tokens: 10,
                ..TokenUsage::default()
            },
        }),
    }
}

fn write_conversation(root: &Path, workspace_dir: &str, id: &str, messages: Vec<ChatMessage>) {
    let conversation = Conversation {
        id: id.into(),
        workspace: Some("/work/demo".into()),
        messages,
        ..Conversation::default()
    };
    let dir = root.join(workspace_dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    let json = serde_json::to_vec(&conversation).expect("json");
    std::fs::write(dir.join(format!("{id}.json")), json).expect("write");
}

#[test]
fn backfill_imports_stored_replies_once() {
    let dir = TestDir::new("usage-backfill");
    let chats = dir.path().join("conversations");
    write_conversation(&chats, "ws1", "c1", vec![reply("a1", 100), reply("a2", 50)]);
    write_conversation(&chats, ".trash/ws1", "c0", vec![reply("a9", 7)]);
    let usage = tracker(&dir);
    assert_eq!(usage.backfill_once(&chats).expect("backfill"), 3);

    write_conversation(&chats, "ws1", "c2", vec![reply("a3", 1)]);
    assert_eq!(
        usage.backfill_once(&chats).expect("again"),
        0,
        "the marker stops it"
    );
    let ledger = LedgerFiles::new(dir.path().join("usage"));
    assert_eq!(ledger.load("2000-01").len(), 3);

    // A restart loads the ledger instead of importing again.
    let restarted = tracker(&dir);
    restarted.start(&Settings::default(), &chats);
    assert_eq!(restarted.summary(0).today.tokens.input_tokens, 157);
}

#[test]
fn openrouter_prices_survive_a_restart() {
    let dir = TestDir::new("usage-openrouter");
    let usage = tracker(&dir);
    let mut model = crate::providers::ModelInfo::named("openai/gpt-x", None);
    model.pricing = Some(serde_json::json!({ "prompt": "0.000002", "completion": "0.00001" }));
    usage.note_models("or", "openrouter", std::slice::from_ref(&model));
    assert!(usage.price_for("or", "openai/gpt-x").is_some());

    let restarted = tracker(&dir);
    assert!(
        restarted.price_for("or", "openai/gpt-x").is_none(),
        "not loaded yet"
    );
    restarted.start(&Settings::default(), &dir.path().join("conversations"));
    let price = restarted
        .price_for("or", "openai/gpt-x")
        .expect("cached price");
    assert!((price.input - 2.0).abs() < 1e-9 && (price.output - 10.0).abs() < 1e-9);
}

#[test]
fn records_are_deduplicated_and_announced_per_panel() {
    let dir = TestDir::new("usage-record");
    let usage = tracker(&dir);
    let seen = events(&usage);
    let mut one = sample(now_ms(), Source::Api, "claude-sonnet-4-6", 1_000, 10);
    one.id = "chat:c1:a1".into();
    one.panel = Some("chat-1".into());
    assert_eq!(
        usage
            .record(vec![one.clone(), one.clone()])
            .expect("record"),
        1
    );
    assert_eq!(usage.record(vec![one]).expect("again"), 0);
    assert_eq!(
        *seen.lock().expect("lock"),
        vec![UsageEvent::Changed {
            panels: vec!["chat-1".into()]
        }]
    );
    let totals = usage.panel_totals("chat-1", 0);
    assert_eq!((totals.tokens.input_tokens, totals.requests), (1_000, 1));
}

#[test]
fn the_hard_stop_lets_exactly_one_send_through_per_allow_once() {
    let dir = TestDir::new("usage-budget");
    let usage = tracker(&dir);
    usage.apply_settings(&UsageSettings {
        track_cli: false,
        budgets: Budgets {
            daily: Some(1.0),
            hard_stop: true,
            ..Budgets::default()
        },
        ..UsageSettings::default()
    });
    let seen = events(&usage);
    assert!(usage.check_send("p1", None).is_ok());
    // claude-sonnet-4-6 input: $3 per 1M tokens -> $1.50.
    let spend = sample(now_ms(), Source::Api, "claude-sonnet-4-6", 500_000, 0);
    usage.record(vec![spend.clone(), spend]).expect("record");
    let budget_events = seen
        .lock()
        .expect("lock")
        .iter()
        .filter(|e| matches!(e, UsageEvent::Budget { .. }))
        .count();
    assert_eq!(budget_events, 1, "one warning at 100%, not one per record");

    assert!(matches!(
        usage.check_send("p1", None),
        Err(AppError::Budget(_))
    ));
    let grant = usage.allow_once("p1");
    assert!(usage.check_send("p1", Some(&grant.token)).is_ok());
    assert!(
        matches!(
            usage.check_send("p1", Some(&grant.token)),
            Err(AppError::Budget(_))
        ),
        "the token is single use"
    );
    let status = &usage.budgets(0)[0];
    assert_eq!((status.level, status.limit), (100, 1.0));
}

#[test]
fn cli_logs_are_read_once_and_matched_to_their_panel() {
    let dir = TestDir::new("usage-cli");
    let roots = CliRoots {
        claude: Some(dir.path().join("claude/projects")),
        ..CliRoots::default()
    };
    let usage = UsageTracker::new(dir.path().join("usage"), roots);
    lock(&usage.inner.state).panels.start(PanelSession {
        panel: "terminal-1".into(),
        kind: CliKind::Claude,
        cwd: "/work/demo".into(),
        workspace: Some("/work/demo".into()),
        started: 0,
        ended: None,
    });
    let file = dir.path().join("claude/projects/-work-demo/s1.jsonl");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, include_str!("cli/fixtures/claude.jsonl")).expect("write");
    usage.ingest(std::slice::from_ref(&file));
    usage.ingest(std::slice::from_ref(&file));

    let ledger = LedgerFiles::new(dir.path().join("usage"));
    let records = ledger.load("2000-01");
    assert_eq!(records.len(), 2, "two API messages, each counted once");
    assert!(records.iter().all(|r| r.source == Source::ClaudeCode
        && r.panel.as_deref() == Some("terminal-1")
        && r.provider == "claude-code"));
    assert!(dir.path().join("usage/cli-offsets.json").exists());
}
