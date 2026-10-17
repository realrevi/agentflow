use super::*;

#[test]
fn claude_code_counts_each_message_once_and_skips_bad_lines() {
    let dir = TestDir::new("cli-claude");
    let path = dir.path().join("projects/-work-demo/s1.jsonl");
    write(&path, CLAUDE);
    let mut store = OffsetStore::default();
    let mut seen = HashSet::new();
    let usage = read_new(CliKind::Claude, &path, &mut store, &mut seen);
    let ids: Vec<&str> = usage.iter().map(|u| u.id.as_str()).collect();
    assert_eq!(
        ids,
        ["msg_1", "msg_2"],
        "duplicate, synthetic and broken lines skipped"
    );
    let first = &usage[0];
    assert_eq!(first.model.as_deref(), Some("claude-sonnet-4-6"));
    assert_eq!(first.cwd.as_deref(), Some("/work/demo"));
    assert_eq!(first.tokens.input_tokens, 3);
    assert_eq!(first.tokens.output_tokens, 200);
    assert_eq!(first.tokens.cache_read_tokens, 20_000);
    assert_eq!(first.tokens.cache_write_tokens, 1_000);
    assert!(first.at > 0);
}

#[test]
fn codex_prefers_usage_records_and_converts_cached_input() {
    let dir = TestDir::new("cli-codex");
    let path = dir
        .path()
        .join("sessions/2026/10/05/rollout-2026-10-05T19-46-24-t1.jsonl");
    write(&path, CODEX);
    let mut store = OffsetStore::default();
    let mut seen = HashSet::new();
    let usage = read_new(CliKind::Codex, &path, &mut store, &mut seen);
    assert_eq!(usage.len(), 2, "token_count repeats the record: ignored");
    assert_eq!(usage[0].id, "resp_1");
    assert_eq!(
        usage[0].model.as_deref(),
        Some("gpt-6-sol"),
        "from turn_context"
    );
    assert_eq!(
        usage[0].cwd.as_deref(),
        Some("/work/demo"),
        "from session_meta"
    );
    assert_eq!(usage[0].tokens.input_tokens, 27_669 - 22_016);
    assert_eq!(usage[0].tokens.cache_read_tokens, 22_016);
    assert_eq!(
        usage[1].tokens.output_tokens, 500,
        "reasoning is part of output"
    );
}

#[test]
fn old_codex_files_use_token_count_events_without_double_counting() {
    let dir = TestDir::new("cli-codex-old");
    let path = dir
        .path()
        .join("sessions/2026/04/26/rollout-2026-04-26T00-08-12-t0.jsonl");
    write(&path, CODEX_OLD);
    let mut store = OffsetStore::default();
    let mut seen = HashSet::new();
    let usage = read_new(CliKind::Codex, &path, &mut store, &mut seen);
    assert_eq!(
        usage.len(),
        2,
        "a repeated total and a rate-limit-only event are skipped"
    );
    assert_eq!(usage[0].tokens.input_tokens, 800);
    assert_eq!(usage[1].tokens.input_tokens, 1100);
    assert_eq!(usage[1].model.as_deref(), Some("gpt-5"));
}

#[test]
fn gemini_counts_a_message_once_when_its_tokens_arrive() {
    let dir = TestDir::new("cli-gemini");
    let path = dir
        .path()
        .join("tmp/demo/chats/session-2026-10-17T09-00-g1.jsonl");
    write(&path, GEMINI);
    let mut store = OffsetStore::default();
    let mut seen = HashSet::new();
    let usage = read_new(CliKind::Gemini, &path, &mut store, &mut seen);
    assert_eq!(usage.len(), 2);
    let m2 = &usage[0];
    assert_eq!(m2.id, "m2");
    assert_eq!(m2.tokens.input_tokens, 4_000);
    assert_eq!(m2.tokens.cache_read_tokens, 8_000);
    assert_eq!(
        m2.tokens.output_tokens, 420,
        "thoughts are billed as output"
    );
    assert_eq!(usage[1].model.as_deref(), Some("gemini-2.5-flash"));

    // Legacy whole-file sessions are re-read on change; ids keep counts right.
    let legacy = dir
        .path()
        .join("tmp/demo/chats/session-2025-01-01T00-00-old.json");
    write(
        &legacy,
        r#"{"sessionId":"x","messages":[{"id":"o1","type":"gemini","timestamp":"2025-01-01T00:00:00Z","model":"gemini-2.5-pro","tokens":{"input":10,"output":5,"cached":0}}]}"#,
    );
    assert_eq!(
        read_new(CliKind::Gemini, &legacy, &mut store, &mut seen).len(),
        1
    );
    assert!(read_new(CliKind::Gemini, &legacy, &mut store, &mut seen).is_empty());
}
