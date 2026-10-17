//! Tool calling in both adapters: schema conversion, request shapes (tool calls, results,
//! reasoning replay) and streaming assembly from fixtures.

use serde_json::json;

use super::decoders::decode;
use crate::providers::anthropic::{request_body as anthropic_body, AnthropicDecoder};
use crate::providers::openai::{request_body as openai_body, OpenAiDecoder, Preset};
use crate::providers::types::{
    ChatRequest, ChatRole, ChatTurn, Part, Reasoning, ToolCall, ToolResult, ToolSpec, TurnBlock,
};

fn read_tool() -> ToolSpec {
    ToolSpec {
        name: "read_file".into(),
        description: "Read a file".into(),
        schema: json!({ "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] }),
    }
}

/// user -> assistant (reasoning, text, call) -> tool result (error) -> (new prompt).
fn agent_chat() -> ChatRequest {
    let reasoning = Reasoning {
        text: "think".into(),
        signature: Some("sig".into()),
        details: Some(json!([{ "type": "reasoning.text", "text": "think", "index": 0 }])),
        ..Reasoning::default()
    };
    ChatRequest {
        model: "m".into(),
        system: "sys".into(),
        turns: vec![
            ChatTurn::user("Read a.ts"),
            ChatTurn {
                role: ChatRole::Assistant,
                blocks: vec![
                    TurnBlock::part(Part::Reasoning(reasoning)),
                    TurnBlock::text("Reading."),
                    TurnBlock::part(Part::ToolCall(ToolCall {
                        id: "call_1".into(),
                        name: "read_file".into(),
                        input: json!({ "path": "a.ts" }),
                    })),
                ],
            },
            ChatTurn {
                role: ChatRole::User,
                blocks: vec![
                    TurnBlock::part(Part::ToolResult(ToolResult {
                        call_id: "call_1".into(),
                        content: "no such file".into(),
                        is_error: true,
                    })),
                    TurnBlock::text("Try b.ts"),
                ],
            },
        ],
        max_tokens: 100,
        temperature: None,
        caching: false,
        tools: vec![read_tool()],
    }
}

#[test]
fn anthropic_request_carries_tools_calls_results_and_thinking() {
    let body = anthropic_body(&agent_chat());
    assert_eq!(body["tools"][0]["name"], "read_file");
    assert_eq!(body["tools"][0]["input_schema"]["required"][0], "path");
    let assistant = &body["messages"][1]["content"];
    assert_eq!(
        assistant[0],
        json!({ "type": "thinking", "thinking": "think", "signature": "sig" })
    );
    assert_eq!(assistant[1], json!({ "type": "text", "text": "Reading." }));
    assert_eq!(
        assistant[2],
        json!({ "type": "tool_use", "id": "call_1", "name": "read_file", "input": { "path": "a.ts" } })
    );
    let results = &body["messages"][2]["content"];
    assert_eq!(
        results[0],
        json!({ "type": "tool_result", "tool_use_id": "call_1", "content": "no such file", "is_error": true })
    );
    assert_eq!(results[1]["text"], "Try b.ts");
}

#[test]
fn openai_request_carries_tools_calls_and_tool_messages() {
    let body = openai_body(&agent_chat(), Preset::OpenAi);
    assert_eq!(
        body["tools"][0],
        json!({ "type": "function", "function": {
            "name": "read_file", "description": "Read a file",
            "parameters": read_tool().schema } })
    );
    let messages = body["messages"].as_array().expect("messages");
    let assistant = &messages[2];
    assert_eq!(assistant["content"], "Reading.");
    assert_eq!(assistant["tool_calls"][0]["id"], "call_1");
    assert_eq!(assistant["tool_calls"][0]["function"]["name"], "read_file");
    assert_eq!(
        assistant["tool_calls"][0]["function"]["arguments"],
        "{\"path\":\"a.ts\"}"
    );
    assert!(
        assistant.get("reasoning_content").is_none(),
        "OpenAI gets no reasoning field"
    );
    assert_eq!(
        messages[3],
        json!({ "role": "tool", "tool_call_id": "call_1", "content": "Error: no such file" })
    );
    assert_eq!(
        messages[4],
        json!({ "role": "user", "content": "Try b.ts" })
    );
}

#[test]
fn reasoning_goes_back_where_the_provider_needs_it() {
    let deepseek = openai_body(&agent_chat(), Preset::DeepSeek);
    assert_eq!(deepseek["messages"][2]["reasoning_content"], "think");
    let custom = openai_body(&agent_chat(), Preset::Custom);
    assert_eq!(custom["messages"][2]["reasoning_content"], "think");
    let openrouter = openai_body(&agent_chat(), Preset::OpenRouter);
    assert_eq!(
        openrouter["messages"][2]["reasoning_details"][0]["text"],
        "think"
    );
    // Tool calls only: content is null.
    let mut chat = agent_chat();
    chat.turns[1]
        .blocks
        .retain(|b| matches!(b.part, Part::ToolCall(_)));
    let body = openai_body(&chat, Preset::OpenAi);
    assert!(body["messages"][2]["content"].is_null());
}

#[test]
fn anthropic_stream_assembles_tool_input_and_keeps_the_signature() {
    let (sink, end) = decode(
        &mut AnthropicDecoder::default(),
        include_str!("../fixtures/anthropic_tool_use.sse"),
    );
    assert_eq!(end.expect("stop"), "tool_use");
    assert_eq!(sink.text, "I'll read both files.");
    assert_eq!(sink.reasoning_text, "Both files are needed.");
    assert_eq!(sink.reasoning.len(), 1);
    assert_eq!(sink.reasoning[0].signature.as_deref(), Some("sig-abc"));
    let calls: Vec<(&str, &str, serde_json::Value)> = sink
        .calls
        .iter()
        .map(|c| (c.id.as_str(), c.name.as_str(), c.input.clone()))
        .collect();
    assert_eq!(
        calls,
        vec![
            ("toolu_01", "read_file", json!({ "path": "src/a.ts" })),
            (
                "toolu_02",
                "read_file",
                json!({ "path": "b.ts", "limit": 20 })
            ),
        ]
    );
}

#[test]
fn openai_stream_assembles_parallel_calls_by_index() {
    let (sink, end) = decode(
        &mut OpenAiDecoder::default(),
        include_str!("../fixtures/openai_tool_calls.sse"),
    );
    assert_eq!(end.expect("stop"), "tool_calls");
    assert_eq!(sink.text, "Checking.");
    assert_eq!(sink.reasoning_text, "Need the file and the list.");
    assert_eq!(sink.reasoning[0].text, "Need the file and the list.");
    assert_eq!(sink.calls.len(), 2);
    assert_eq!(sink.calls[0].id, "call_a");
    assert_eq!(sink.calls[0].input, json!({ "path": "src/a.ts" }));
    assert_eq!(sink.calls[1].id, "call_b");
    assert_eq!(sink.calls[1].name, "glob");
    assert_eq!(sink.calls[1].input, json!({ "pattern": "**/*.rs" }));
    assert_eq!(sink.last_usage().cache_read_tokens, 128);
}

#[test]
fn reasoning_deltas_from_both_openai_styles() {
    let (sink, _) = decode(
        &mut OpenAiDecoder::default(),
        include_str!("../fixtures/deepseek_reasoning.sse"),
    );
    assert!(
        sink.reasoning_text.ends_with(" memory"),
        "{}",
        sink.reasoning_text
    );
    assert!(sink.calls.is_empty());

    let (sink, end) = decode(
        &mut OpenAiDecoder::default(),
        include_str!("../fixtures/openrouter_reasoning.sse"),
    );
    assert_eq!(end.expect("stop"), "stop");
    assert_eq!(sink.text, "Done.");
    assert_eq!(sink.reasoning_text, "Thinking.");
    let details = sink.reasoning[0].details.as_ref().expect("details");
    assert_eq!(
        details,
        &json!([{ "type": "reasoning.text", "text": "Thinking.", "format": "anthropic-claude-v1",
                  "index": 0, "signature": "sig-or" }])
    );
}

#[test]
fn invalid_tool_json_is_kept_raw() {
    let input = crate::providers::types::parse_tool_input("{\"path\": ");
    assert_eq!(input, json!({ "_raw": "{\"path\": " }));
    assert_eq!(crate::providers::types::parse_tool_input(""), json!({}));
}
