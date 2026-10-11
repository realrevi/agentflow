//! Fixture streams (shapes from each provider's docs) through the two decoders.

use super::Collect;
use crate::providers::anthropic::AnthropicDecoder;
use crate::providers::error::{ErrorKind, ProviderError};
use crate::providers::http::Decoder;
use crate::providers::openai::OpenAiDecoder;
use crate::providers::sse::SseParser;
use crate::providers::types::TokenUsage;

/// Feeds a whole stream (split at awkward points) and returns the reply and how it ended.
fn decode(decoder: &mut dyn Decoder, stream: &str) -> (Collect, Result<String, ProviderError>) {
    let mut sink = Collect::default();
    let mut parser = SseParser::new();
    let bytes = stream.as_bytes();
    for chunk in bytes.chunks(37) {
        for event in parser.push(chunk).expect("push") {
            match decoder.event(&event, &mut sink) {
                Ok(Some(stop)) => return (sink, Ok(stop)),
                Ok(None) => {}
                Err(err) => return (sink, Err(err)),
            }
        }
    }
    if let Some(event) = parser.finish() {
        match decoder.event(&event, &mut sink) {
            Ok(Some(stop)) => return (sink, Ok(stop)),
            Ok(None) => {}
            Err(err) => return (sink, Err(err)),
        }
    }
    let end = decoder.finish();
    (sink, end)
}

fn usage(input: u64, output: u64, cache_read: u64, cache_write: u64) -> TokenUsage {
    TokenUsage {
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: cache_read,
        cache_write_tokens: cache_write,
    }
}

#[test]
fn anthropic_basic_stream() {
    let (sink, end) = decode(
        &mut AnthropicDecoder::default(),
        include_str!("../fixtures/anthropic_basic.sse"),
    );
    assert_eq!(end.expect("stop"), "end_turn");
    assert_eq!(sink.text, "Hello!");
    assert_eq!(sink.last_usage(), usage(25, 15, 0, 0));
}

#[test]
fn anthropic_skips_thinking_and_reports_cache_usage() {
    let (sink, end) = decode(
        &mut AnthropicDecoder::default(),
        include_str!("../fixtures/anthropic_thinking_cache.sse"),
    );
    assert_eq!(end.expect("stop"), "end_turn");
    assert_eq!(sink.text, "Hi there! How can I help?");
    assert!(!sink.text.contains("secret reasoning"));
    // message_start already knew the input and cache counts; message_delta updates output.
    assert_eq!(sink.usage.first().copied(), Some(usage(12, 1, 2048, 100)));
    assert_eq!(sink.last_usage(), usage(12, 42, 2048, 100));
}

#[test]
fn anthropic_mid_stream_error() {
    let (sink, end) = decode(
        &mut AnthropicDecoder::default(),
        include_str!("../fixtures/anthropic_error.sse"),
    );
    let err = end.expect_err("error event");
    assert_eq!(err.kind, ErrorKind::Overloaded);
    assert_eq!(err.message, "Overloaded");
    assert_eq!(sink.text, "Partial");
    assert_eq!(sink.last_usage().input_tokens, 25);
}

#[test]
fn anthropic_refusal_is_a_stop_reason() {
    let (sink, end) = decode(
        &mut AnthropicDecoder::default(),
        include_str!("../fixtures/anthropic_refusal.sse"),
    );
    assert_eq!(end.expect("stop"), "refusal");
    assert_eq!(sink.text, "Sure, here");
}

#[test]
fn anthropic_stream_cut_before_message_stop_is_an_error() {
    let full = include_str!("../fixtures/anthropic_basic.sse");
    let cut = &full[..full.find("event: message_delta").expect("delta")];
    let (sink, end) = decode(&mut AnthropicDecoder::default(), cut);
    assert_eq!(sink.text, "Hello!");
    assert_eq!(end.expect_err("cut").kind, ErrorKind::Network);
}

#[test]
fn openai_usage_chunk_with_cache_fields() {
    let (sink, end) = decode(
        &mut OpenAiDecoder::default(),
        include_str!("../fixtures/openai_usage.sse"),
    );
    assert_eq!(end.expect("stop"), "stop");
    assert_eq!(sink.text, "Hello! How can I help you today?");
    // prompt_tokens includes the cached ones: 1219 - 1024 uncached.
    assert_eq!(sink.last_usage(), usage(195, 10, 1024, 64));
}

#[test]
fn openai_mid_stream_error_frame() {
    let (sink, end) = decode(
        &mut OpenAiDecoder::default(),
        include_str!("../fixtures/openai_error.sse"),
    );
    let err = end.expect_err("error frame");
    assert_eq!(err.kind, ErrorKind::Server);
    assert!(err.message.starts_with("The server had an error"));
    assert_eq!(sink.text, "Hel");
}

#[test]
fn deepseek_reasoning_is_skipped_and_cache_hits_counted() {
    let (sink, end) = decode(
        &mut OpenAiDecoder::default(),
        include_str!("../fixtures/deepseek_reasoning.sse"),
    );
    assert_eq!(end.expect("stop"), "length");
    assert_eq!(sink.text, "Use a profiler.");
    assert_eq!(sink.last_usage(), usage(6, 81, 40, 0));
}

#[test]
fn gemini_cumulative_usage_and_thinking_tokens() {
    let (sink, end) = decode(
        &mut OpenAiDecoder::default(),
        include_str!("../fixtures/gemini_cumulative.sse"),
    );
    assert_eq!(end.expect("stop"), "stop");
    assert_eq!(sink.text, "Hi there! How can I help you today?");
    assert_eq!(sink.usage.len(), 3, "every chunk reports running totals");
    // Thinking is only in total_tokens: output = 115 - 12.
    assert_eq!(sink.last_usage(), usage(8, 103, 4, 0));
}

#[test]
fn openrouter_usage_chunk_repeats_the_finish_reason() {
    let (sink, end) = decode(
        &mut OpenAiDecoder::default(),
        include_str!("../fixtures/openrouter_usage.sse"),
    );
    assert_eq!(end.expect("stop"), "stop");
    assert_eq!(sink.text, "Hello there!");
    assert_eq!(sink.last_usage(), usage(21, 4, 10318, 0));
}

#[test]
fn openrouter_mid_stream_error_uses_its_status_code() {
    let (sink, end) = decode(
        &mut OpenAiDecoder::default(),
        include_str!("../fixtures/openrouter_error.sse"),
    );
    let err = end.expect_err("error chunk");
    assert_eq!(err.kind, ErrorKind::RateLimit);
    assert_eq!(err.message, "Rate limit exceeded");
    assert_eq!(sink.text, "Partial answer");

    // The streaming page's variant has a string code and no metadata.
    let string_code = "data: {\"id\":\"cmpl-abc123\",\"provider\":\"openai\",\"error\":{\"code\":\"server_error\",\"message\":\"Provider disconnected unexpectedly\"},\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\"},\"finish_reason\":\"error\"}]}\n\n";
    let (_, end) = decode(&mut OpenAiDecoder::default(), string_code);
    let err = end.expect_err("string code");
    assert_eq!(err.kind, ErrorKind::Server);
    assert_eq!(err.message, "Provider disconnected unexpectedly");
}

#[test]
fn groq_usage_on_the_finish_chunk() {
    let (sink, end) = decode(
        &mut OpenAiDecoder::default(),
        include_str!("../fixtures/groq_usage.sse"),
    );
    assert_eq!(end.expect("stop"), "stop");
    assert_eq!(sink.text, "Hello! How can I help?");
    assert_eq!(sink.last_usage(), usage(45, 8, 0, 0));
}

#[test]
fn openai_compatible_edge_cases() {
    // A finished reply without [DONE] still counts; an unfinished one does not.
    let finished = "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\n";
    let (_, end) = decode(&mut OpenAiDecoder::default(), finished);
    assert_eq!(end.expect("stop"), "stop");
    let cut = "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"o\"},\"finish_reason\":null}]}\n\n";
    let (_, end) = decode(&mut OpenAiDecoder::default(), cut);
    assert_eq!(end.expect_err("cut").kind, ErrorKind::Network);

    // DeepSeek gives up with a special finish reason; Google sends errors as arrays.
    let gave_up = "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\"},\"finish_reason\":\"insufficient_system_resource\"}]}\n\n";
    let (_, end) = decode(&mut OpenAiDecoder::default(), gave_up);
    assert_eq!(end.expect_err("gave up").kind, ErrorKind::Server);
    let array = "data: [{\"error\":{\"code\":503,\"message\":\"The model is overloaded.\",\"status\":\"UNAVAILABLE\"}}]\n\n";
    let (_, end) = decode(&mut OpenAiDecoder::default(), array);
    assert_eq!(end.expect_err("array").kind, ErrorKind::Overloaded);

    // Groq's early-stop string, and a refusal shown as the reply.
    let groq = "data: {\"choices\":[],\"x_groq\":{\"error\":\"stream stopped\"}}\n\n";
    let (_, end) = decode(&mut OpenAiDecoder::default(), groq);
    assert_eq!(end.expect_err("groq").message, "stream stopped");
    let refusal = "data: {\"choices\":[{\"index\":0,\"delta\":{\"refusal\":\"I can't help with that.\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
    let (sink, end) = decode(&mut OpenAiDecoder::default(), refusal);
    assert_eq!(sink.text, "I can't help with that.");
    assert_eq!(end.expect("stop"), "stop");
}
