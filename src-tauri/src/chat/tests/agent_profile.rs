//! AgentFlow agent profiles in the loop: the tool list only narrows what the mode offers,
//! unlisted tools are refused without asking, the role reaches the system prompt; and
//! "Create with AI" retries once on an invalid template.

use serde_json::json;

use super::agent::{agent_setup, permission_count, run_answering, text_reply, tool_call};
use super::*;
use crate::agents::{generate, GenerateRequest};
use crate::conversations::{PermissionMode, ProfileSnapshot};

#[test]
fn a_profile_narrows_tools_and_adds_its_role() {
    let server = MockServer::start(vec![
        tool_call(
            "c1",
            "write_file",
            json!({ "path": "x.txt", "content": "no" }),
        ),
        text_reply("Done."),
    ]);
    let setup = agent_setup(&server.base);
    setup
        .base
        .store
        .update_agent(Some(&setup.ws), &setup.conversation, |agent| {
            agent.profile = Some(ProfileSnapshot {
                id: "reviewer".into(),
                name: "Reviewer".into(),
                instructions: "Only review.".into(),
                tools: Some(vec![
                    "read_file".into(),
                    "search".into(),
                    "write_file".into(),
                ]),
            });
            agent.permission_mode = Some(PermissionMode::ReadOnly);
        })
        .expect("profile");
    let (_, rx) = setup.send("fix it");
    let events = run_answering(&setup, &rx, || panic!("nothing may ask"));
    assert_eq!(permission_count(&events), 0);
    let first = server.requests()[0].json();
    let names: Vec<&str> = first["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|t| t["function"]["name"].as_str())
        .collect();
    // write_file is in the profile but Read-only does not allow it: never widened.
    assert_eq!(names, ["read_file", "search"]);
    let system = first["messages"][0]["content"].as_str().unwrap_or("");
    assert!(
        system.contains("## Your role: Reviewer\n\nOnly review."),
        "{system}"
    );
    assert!(!setup.file("x.txt").exists());
    assert_eq!(
        PermissionMode::AutoEdit.stricter(PermissionMode::Ask),
        PermissionMode::Ask
    );
    assert_eq!(
        PermissionMode::ReadOnly.stricter(PermissionMode::AutoEdit),
        PermissionMode::ReadOnly
    );
}

#[test]
fn a_tool_outside_the_profile_is_refused() {
    let server = MockServer::start(vec![
        tool_call("c1", "list_dir", json!({})),
        text_reply("ok"),
    ]);
    let setup = agent_setup(&server.base);
    setup
        .base
        .store
        .update_agent(Some(&setup.ws), &setup.conversation, |agent| {
            agent.profile = Some(ProfileSnapshot {
                tools: Some(vec!["read_file".into()]),
                ..ProfileSnapshot::default()
            });
        })
        .expect("profile");
    let (_, rx) = setup.send("look");
    run_answering(&setup, &rx, || panic!("nothing may ask"));
    let results = super::agent::tool_results(&server, 1);
    assert!(results[0]["content"]
        .as_str()
        .unwrap_or("")
        .contains("not available to this agent"));
}

fn template_json(id: &str) -> String {
    json!({
        "schema": 1, "id": id, "name": "Lint Fixer", "description": "Fixes lint errors.",
        "version": "1.0.0", "instructions": "Fix lint in {{workspaceName}}."
    })
    .to_string()
}

#[test]
fn create_with_ai_retries_once_with_the_validation_errors() {
    let server = MockServer::start(vec![
        text_reply(&template_json("Bad Id")),
        text_reply(&format!("```json\n{}\n```", template_json("lint-fixer"))),
    ]);
    let setup = setup(&server.base, "openai", "custom");
    let request = GenerateRequest {
        workspace: WS.map(str::to_owned),
        provider_id: "p1".into(),
        model: "model-x".into(),
        description: "an agent that fixes lint errors".into(),
        language: "tr".into(),
        base: None,
    };
    let result = tauri::async_runtime::block_on(generate(setup.deps(), request)).expect("draft");
    assert_eq!(result.attempts, 2);
    assert_eq!(result.template.id, "lint-fixer");
    let requests = server.requests();
    let retry = requests[1].json();
    let last = retry["messages"]
        .as_array()
        .and_then(|m| m.last())
        .cloned()
        .unwrap_or_default();
    assert!(
        last["content"].as_str().unwrap_or("").contains("id:"),
        "{last}"
    );
    let system = requests[0].json()["messages"][0]["content"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    assert!(system.contains("language \"tr\""));
    // Both calls are in the ledger.
    assert_eq!(setup.tracker.summary(0).today.requests, 2);
}

#[test]
fn create_with_ai_gives_up_after_the_retry() {
    let server = MockServer::start(vec![text_reply("no json here"), text_reply("still none")]);
    let setup = setup(&server.base, "openai", "custom");
    let request = GenerateRequest {
        workspace: None,
        provider_id: "p1".into(),
        model: "model-x".into(),
        description: "x".into(),
        language: "en".into(),
        base: None,
    };
    let err = tauri::async_runtime::block_on(generate(setup.deps(), request)).unwrap_err();
    assert!(err.to_string().contains("no JSON object"), "{err}");
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn create_with_ai_handles_a_reply_cut_off_at_the_output_limit() {
    let chunk = json!({ "choices": [{ "index": 0, "delta": { "content": "{\"schema\": 1, \"id\": \"a11y" },
        "finish_reason": "length" }],
        "usage": { "prompt_tokens": 10, "completion_tokens": 4096, "total_tokens": 4106 } });
    let cut = Reply::sse(&format!("data: {chunk}\n\ndata: [DONE]\n\n"));
    let server = MockServer::start(vec![cut, text_reply(&template_json("a11y-checker"))]);
    let setup = setup(&server.base, "openai", "custom");
    let request = GenerateRequest {
        workspace: None,
        provider_id: "p1".into(),
        model: "model-x".into(),
        description: "accessibility checker".into(),
        language: "en".into(),
        base: None,
    };
    let result = tauri::async_runtime::block_on(generate(setup.deps(), request)).expect("draft");
    assert_eq!(
        (result.attempts, result.template.id.as_str()),
        (2, "a11y-checker")
    );
    let requests = server.requests();
    let first = requests[0].json();
    let budget = first["max_tokens"]
        .as_u64()
        .or_else(|| first["max_completion_tokens"].as_u64())
        .unwrap_or(0);
    assert!(budget >= 16_000, "output budget {budget}");
    let retry = requests[1].json();
    let last = retry["messages"]
        .as_array()
        .and_then(|m| m.last())
        .cloned()
        .unwrap_or_default();
    assert!(
        last["content"].as_str().unwrap_or("").contains("cut off"),
        "{last}"
    );
}

#[test]
fn customize_with_ai_adds_questions_and_keeps_the_original() {
    let reply = json!({
        "questions": [{ "id": "testCmd", "label": "Test command", "type": "text",
                        "default": "pnpm test", "detect": "testCommand" }],
        "projectSpecifics": "Run `{{testCmd}}` before you report."
    });
    let server = MockServer::start(vec![text_reply(&reply.to_string())]);
    let setup = setup(&server.base, "openai", "custom");
    let base: crate::agents::Template =
        serde_json::from_str(&template_json("lint-fixer")).expect("base");
    let request = GenerateRequest {
        workspace: WS.map(str::to_owned),
        provider_id: "p1".into(),
        model: "glm-5.3".into(),
        description: String::new(),
        language: "en".into(),
        base: Some(base.clone()),
    };
    let result = tauri::async_runtime::block_on(generate(setup.deps(), request)).expect("draft");
    let template = result.template;
    assert!(template.instructions.starts_with(&base.instructions));
    assert!(template.instructions.contains("## Project specifics"));
    assert_eq!(template.questions.len(), 1);
    let sent = server.requests()[0].json();
    let user = sent["messages"][1]["content"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    assert!(
        user.contains("Fix lint in {{workspaceName}}."),
        "the original goes to the model"
    );
    assert_eq!(
        setup.tracker.summary(0).today.requests,
        1,
        "usage is in the ledger"
    );
}
