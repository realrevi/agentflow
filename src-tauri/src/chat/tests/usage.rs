//! Every finished reply lands in the usage ledger; the hard budget stop blocks sends.

use super::*;
use crate::error::AppError;
use crate::settings::{Budgets, UsageSettings};

#[test]
fn a_finished_reply_is_written_to_the_ledger() {
    let server = MockServer::start(vec![Reply::sse(include_str!(
        "../../providers/fixtures/anthropic_thinking_cache.sse"
    ))]);
    let setup = setup(&server.base, "anthropic", "anthropic");
    let (started, rx) = setup.send(Some("Hello there"));
    started.expect("started");
    let events = collect(&rx);
    let Some(ChatEvent::Stop { message, .. }) = events.last() else {
        panic!("no stop: {events:?}");
    };
    let message = message.as_ref().expect("reply");

    let totals = setup.tracker.panel_totals("chat-1", 0);
    assert_eq!(totals.requests, 1);
    assert_eq!(totals.tokens.output_tokens, 42);
    assert_eq!(totals.tokens.cache_read_tokens, 2048);
    let summary = setup.tracker.summary(0);
    let group = &summary.workspaces[0];
    assert_eq!(
        (group.key.as_str(), group.provider.as_str()),
        ("/work/demo", "p1")
    );
    // The backfill id scheme: the same reply is never counted twice.
    let stored = setup.store.get(WS, &setup.conversation).expect("get");
    let record = crate::tokens::chat_record(
        &setup.conversation,
        &message.id,
        WS,
        None,
        stored.messages[1].usage.as_ref().expect("usage"),
    );
    assert_eq!(setup.tracker.record(vec![record]).expect("record"), 0);
}

#[test]
fn the_hard_stop_refuses_a_send_before_anything_happens() {
    let server = MockServer::start(vec![Reply::sse(include_str!(
        "../../providers/fixtures/anthropic_basic.sse"
    ))]);
    let setup = setup(&server.base, "anthropic", "anthropic");
    setup.tracker.apply_settings(&UsageSettings {
        track_cli: false,
        free_providers: Vec::new(),
        budgets: Budgets {
            monthly: Some(0.000_001),
            hard_stop: true,
            ..Budgets::default()
        },
        prices: vec![crate::settings::PriceOverride {
            model: "model-x".into(),
            input: 1.0,
            output: 1.0,
            ..Default::default()
        }],
        ..UsageSettings::default()
    });
    let mut spend = crate::tokens::chat_record(
        "c-old",
        "a-old",
        WS,
        None,
        &crate::conversations::UsageRecord {
            provider_id: "p1".into(),
            preset: "anthropic".into(),
            model: "model-x".into(),
            at: crate::util::now_ms(),
            tokens: crate::providers::TokenUsage {
                input_tokens: 10,
                ..Default::default()
            },
        },
    );
    spend.panel = None;
    setup.tracker.record(vec![spend]).expect("record");

    let (started, _) = setup.send(Some("Hi"));
    assert!(matches!(started, Err(AppError::Budget(_))));
    assert!(server.requests().is_empty(), "nothing was sent");
    let stored = setup.store.get(WS, &setup.conversation).expect("get");
    assert!(stored.messages.is_empty(), "nothing was stored");
}
