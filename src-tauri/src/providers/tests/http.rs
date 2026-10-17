//! Full requests against the local mock server: request shapes per adapter and preset,
//! cancellation, timeouts and error classification.

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::mock::{MockServer, Reply};
use super::Collect;
use crate::providers::anthropic::API_VERSION;
use crate::providers::error::ErrorKind;
use crate::providers::keys::{KeyVault, MemoryStore};
use crate::providers::{
    build_provider, cancel_pair, ChatRequest, ChatTurn, Http, Provider, StreamEnd,
};
use crate::settings::ProviderEntry;

const KEY: &str = "sk-test-0123456789abcdefghij";

fn http() -> Http {
    Http::for_tests(Duration::from_secs(5))
}

fn entry(kind: &str, preset: &str, base: &str) -> ProviderEntry {
    ProviderEntry {
        id: "p1".into(),
        kind: kind.into(),
        preset: preset.into(),
        base_url: base.into(),
        max_tokens: 512,
        ..ProviderEntry::default()
    }
}

fn provider(entry: &ProviderEntry, with_key: bool) -> Box<dyn Provider> {
    let vault = KeyVault::new(Arc::new(MemoryStore::default()));
    if with_key {
        vault.set("p1", KEY).expect("set key");
    }
    build_provider(entry, vault.get("p1").expect("get key")).expect("provider")
}

fn chat() -> ChatRequest {
    ChatRequest {
        model: "model-x".into(),
        system: "Be brief.".into(),
        turns: vec![ChatTurn::user("Hi")],
        max_tokens: 512,
        temperature: None,
        caching: false,
    }
}

fn run(
    provider: &dyn Provider,
    http: &Http,
) -> (Collect, Result<StreamEnd, crate::providers::ProviderError>) {
    let (_handle, cancel) = cancel_pair();
    let mut sink = Collect::default();
    let end =
        tauri::async_runtime::block_on(provider.stream_chat(http, &chat(), &mut sink, &cancel));
    (sink, end)
}

#[test]
fn anthropic_request_and_stream() {
    let server = MockServer::start(vec![Reply::sse(include_str!(
        "../fixtures/anthropic_basic.sse"
    ))]);
    let (sink, end) = run(
        &*provider(&entry("anthropic", "anthropic", &server.base), true),
        &http(),
    );
    assert_eq!(end.expect("stream"), StreamEnd::Stopped("end_turn".into()));
    assert_eq!(sink.text, "Hello!");
    assert!(sink.flushes >= 2, "deltas are flushed per network chunk");

    let request = &server.requests()[0];
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", "/v1/messages")
    );
    assert_eq!(request.headers["x-api-key"], KEY);
    assert_eq!(request.headers["anthropic-version"], API_VERSION);
    assert!(!request.headers.contains_key("authorization"));
    let body = request.json();
    assert_eq!(body["model"], "model-x");
    assert_eq!(body["max_tokens"], 512);
    assert_eq!(body["system"], "Be brief.");
    assert_eq!(body["stream"], true);
    assert_eq!(body["messages"][0]["role"], "user");
    assert!(
        body.get("temperature").is_none(),
        "only sent when the user set one"
    );
}

#[test]
fn openai_compatible_presets_shape_the_request() {
    struct Case {
        preset: &'static str,
        key: bool,
        system_role: &'static str,
        max_field: &'static str,
        stream_options: bool,
    }
    let cases = [
        Case {
            preset: "openai",
            key: true,
            system_role: "developer",
            max_field: "max_completion_tokens",
            stream_options: true,
        },
        Case {
            preset: "deepseek",
            key: true,
            system_role: "system",
            max_field: "max_tokens",
            stream_options: true,
        },
        Case {
            preset: "gemini",
            key: true,
            system_role: "system",
            max_field: "max_tokens",
            stream_options: true,
        },
        Case {
            preset: "groq",
            key: true,
            system_role: "system",
            max_field: "max_completion_tokens",
            stream_options: false,
        },
        Case {
            preset: "openrouter",
            key: true,
            system_role: "system",
            max_field: "max_completion_tokens",
            stream_options: false,
        },
        Case {
            preset: "ollama",
            key: false,
            system_role: "system",
            max_field: "max_tokens",
            stream_options: true,
        },
    ];
    for case in cases {
        let server = MockServer::start(vec![Reply::sse(include_str!(
            "../fixtures/openai_usage.sse"
        ))]);
        let base = format!("{}/v1", server.base);
        let (sink, end) = run(
            &*provider(&entry("openai", case.preset, &base), case.key),
            &http(),
        );
        assert_eq!(
            end.expect("stream"),
            StreamEnd::Stopped("stop".into()),
            "{}",
            case.preset
        );
        assert_eq!(sink.text, "Hello! How can I help you today?");

        let request = &server.requests()[0];
        assert_eq!(request.path, "/v1/chat/completions", "{}", case.preset);
        assert_eq!(
            request.headers.get("authorization").map(String::as_str),
            case.key.then_some("Bearer sk-test-0123456789abcdefghij"),
            "{}",
            case.preset
        );
        let body = request.json();
        assert_eq!(
            body["messages"][0]["role"], case.system_role,
            "{}",
            case.preset
        );
        assert_eq!(body["messages"][1]["content"], "Hi");
        assert_eq!(body[case.max_field], 512, "{}", case.preset);
        assert_eq!(
            body.get("stream_options").is_some(),
            case.stream_options,
            "{}",
            case.preset
        );
        let other = if case.max_field == "max_tokens" {
            "max_completion_tokens"
        } else {
            "max_tokens"
        };
        assert!(body.get(other).is_none(), "{}", case.preset);
        if case.preset == "openai" {
            assert_eq!(body["stream_options"]["include_obfuscation"], false);
        }
        if case.preset == "openrouter" {
            assert_eq!(request.headers["x-openrouter-title"], "AgentFlow");
            assert!(request.headers.contains_key("http-referer"));
        }
    }
}

#[test]
fn cancel_stops_the_stream_promptly() {
    let full = include_str!("../fixtures/anthropic_basic.sse");
    let first = &full[..full.find("event: content_block_stop").expect("cut")];
    let server = MockServer::start(vec![Reply::sse(first).hold(Duration::from_secs(10))]);
    let provider = provider(&entry("anthropic", "anthropic", &server.base), true);
    let (handle, cancel) = cancel_pair();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        handle.cancel();
    });
    let started = Instant::now();
    let mut sink = Collect::default();
    let end =
        tauri::async_runtime::block_on(provider.stream_chat(&http(), &chat(), &mut sink, &cancel));
    canceller.join().expect("canceller");
    assert_eq!(end.expect("cancelled"), StreamEnd::Cancelled);
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "took {:?}",
        started.elapsed()
    );
    assert_eq!(sink.text, "Hello!", "what arrived before Stop is kept");
    let deadline = Instant::now() + Duration::from_secs(3);
    while !server.client_left() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        server.client_left(),
        "the connection was closed, so the provider stops too"
    );
}

#[test]
fn http_errors_are_classified_and_keys_redacted() {
    let gemini_bad_key = r#"[{"error":{"code":400,"message":"Please pass a valid API key","status":"INVALID_ARGUMENT"}}]"#;
    let echo = format!(
        r#"{{"error":{{"message":"Incorrect API key provided: {KEY}.","type":"invalid_request_error","code":"invalid_api_key"}}}}"#
    );
    let server = MockServer::start(vec![
        Reply::json(401, &echo),
        Reply::json(
            429,
            r#"{"error":{"message":"Slow down","type":"rate_limit_error"}}"#,
        )
        .header("retry-after", "7"),
        Reply::json(
            404,
            r#"{"error":{"message":"The model `x` does not exist","code":"model_not_found"}}"#,
        ),
        Reply::json(400, gemini_bad_key),
        Reply::json(
            529,
            r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#,
        ),
        Reply::json(503, "Service Unavailable").header("retry-after", "3"),
        Reply::json(
            400,
            r#"{"error":{"message":"The model `old` has been decommissioned","code":"model_decommissioned"}}"#,
        ),
        Reply::json(302, "").header("location", "https://evil.example/steal"),
    ]);
    let base = format!("{}/v1", server.base);
    let provider = provider(&entry("openai", "openai", &base), true);
    let expect = [
        (ErrorKind::Auth, None),
        (ErrorKind::RateLimit, Some(7)),
        (ErrorKind::InvalidModel, None),
        (ErrorKind::Auth, None),
        (ErrorKind::Overloaded, None),
        (ErrorKind::Overloaded, Some(3)),
        (ErrorKind::InvalidModel, None),
        (ErrorKind::BadRequest, None),
    ];
    for (kind, retry_after) in expect {
        let (_, end) = run(&*provider, &http());
        let err = end.expect_err("error").redacted(Some(KEY));
        assert_eq!(
            (err.kind, err.retry_after),
            (kind, retry_after),
            "{}",
            err.message
        );
        assert!(!err.message.contains("0123456789abcdef"), "{}", err.message);
    }
    assert_eq!(server.requests().len(), 8, "the redirect was not followed");
    let first = run(&*provider, &http());
    assert!(
        first.1.is_err(),
        "no server left: a network error, not a hang"
    );
}

#[test]
fn stalls_time_out_and_dead_servers_fail_fast() {
    let server = MockServer::start(vec![Reply::sse("").stall(Duration::from_secs(5))]);
    let stalled = provider(&entry("anthropic", "anthropic", &server.base), true);
    let (_, end) = run(&*stalled, &Http::for_tests(Duration::from_millis(300)));
    assert_eq!(end.expect_err("timeout").kind, ErrorKind::Timeout);

    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("bind")
        .local_addr()
        .expect("addr")
        .port();
    let dead = provider(
        &entry(
            "anthropic",
            "anthropic",
            &format!("http://127.0.0.1:{port}"),
        ),
        true,
    );
    let (_, end) = run(&*dead, &http());
    assert_eq!(end.expect_err("refused").kind, ErrorKind::Network);
}

#[test]
fn model_lists_are_fetched_and_cleaned() {
    let server = MockServer::start(vec![
        Reply::json(
            200,
            r#"{"data":[{"type":"model","id":"claude-a","display_name":"Claude A"},{"id":"claude-b","display_name":"Claude B"}],"has_more":true,"first_id":"claude-a","last_id":"claude-b"}"#,
        ),
        Reply::json(
            200,
            r#"{"data":[{"id":"claude-c","display_name":"Claude C"}],"has_more":false,"first_id":"claude-c","last_id":"claude-c"}"#,
        ),
    ]);
    let anthropic = provider(&entry("anthropic", "anthropic", &server.base), true);
    let models = tauri::async_runtime::block_on(anthropic.list_models(&http())).expect("models");
    let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["claude-a", "claude-b", "claude-c"]);
    assert_eq!(models[0].name.as_deref(), Some("Claude A"));
    let requests = server.requests();
    assert!(requests[0].path.starts_with("/v1/models?limit=1000"));
    assert!(
        requests[1].path.contains("after_id=claude-b"),
        "{}",
        requests[1].path
    );
    assert_eq!(requests[1].headers["x-api-key"], KEY);

    let server = MockServer::start(vec![Reply::json(
        200,
        r#"{"object":"list","data":[{"id":"models/gemini-x"},{"id":"models/gemini-a"},{"id":"models/gemini-a"},{"id":"models/gemini-old","active":false}]}"#,
    )]);
    let gemini = provider(
        &entry(
            "openai",
            "gemini",
            &format!("{}/v1beta/openai", server.base),
        ),
        true,
    );
    let models = tauri::async_runtime::block_on(gemini.list_models(&http())).expect("models");
    let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(
        ids,
        ["gemini-a", "gemini-x"],
        "prefix stripped, sorted, unique, inactive ones skipped"
    );
    assert_eq!(server.requests()[0].path, "/v1beta/openai/models");
}

#[test]
fn the_factory_enforces_https_and_keys() {
    let vault = KeyVault::new(Arc::new(MemoryStore::default()));
    let no_key = vault.get("p1").expect("get");
    let remote_http = entry("openai", "openai", "http://api.example.com/v1");
    assert!(build_provider(&remote_http, no_key.clone()).is_err());
    let hosted = entry("openai", "deepseek", "https://api.deepseek.com");
    let err = build_provider(&hosted, no_key.clone())
        .err()
        .expect("needs a key");
    assert_eq!(err.kind, ErrorKind::Config);
    let ollama = entry("openai", "ollama", "http://localhost:11434/v1");
    assert!(build_provider(&ollama, no_key.clone()).is_ok());
    let custom_local = entry("openai", "custom", "http://127.0.0.1:1234/v1");
    assert!(build_provider(&custom_local, no_key.clone()).is_ok());
    let anthropic = entry("anthropic", "anthropic", "https://api.anthropic.com");
    assert!(build_provider(&anthropic, no_key).is_err());
}
