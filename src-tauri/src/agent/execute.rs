//! One tool call: the permission decision (asking the user when needed, cancel-aware),
//! then the run, with status events for the character and the chat.

use super::dangerous::dangerous;
use super::permissions::{decide, Category, Decision, PermissionAnswer, PermissionRequest};
use super::run::Turn;
use super::tools::{self, ToolEnv, ToolOutput};
use crate::chat::{ChatEvent, Phase};
use crate::conversations::PermissionMode;
use crate::providers::{ToolCall, ToolResult};
use crate::util::run_blocking;

/// Per-turn state the calls share.
pub struct CallContext<'a> {
    pub env: Option<&'a ToolEnv>,
    pub mode: PermissionMode,
    /// Tools allowed "for this conversation" (grows when the user picks that).
    pub allowed: &'a mut Vec<String>,
}

pub async fn execute(
    turn: &Turn,
    send: &mut (dyn FnMut(ChatEvent) + Send),
    context: &mut CallContext<'_>,
    call: &ToolCall,
) -> ToolResult {
    let output = decide_and_run(turn, send, context, call).await;
    ToolResult {
        call_id: call.id.clone(),
        content: output.content,
        is_error: output.is_error,
    }
}

async fn decide_and_run(
    turn: &Turn,
    send: &mut (dyn FnMut(ChatEvent) + Send),
    context: &mut CallContext<'_>,
    call: &ToolCall,
) -> ToolOutput {
    if turn.cancel.is_cancelled() {
        return cancelled();
    }
    let (Some(env), Some(category)) = (context.env, tools::category(&call.name)) else {
        return ToolOutput::error(format!("The tool \"{}\" is not available here.", call.name));
    };
    let danger = match category {
        Category::Command => call.input["command"].as_str().and_then(dangerous),
        _ => None,
    };
    match decide(
        context.mode,
        category,
        &call.name,
        context.allowed,
        danger.is_some(),
    ) {
        Decision::Refuse => {
            return ToolOutput::error("This tool is not available in read-only mode.");
        }
        Decision::Ask => {
            let request = PermissionRequest {
                id: 0,
                call_id: call.id.clone(),
                tool: call.name.clone(),
                input: call.input.clone(),
                category,
                dangerous: danger.map(str::to_owned),
                preview: tools::preview(env, call),
            };
            match ask(turn, send, request).await {
                None => return cancelled(),
                Some(PermissionAnswer::Deny { reason }) => return denied(reason),
                Some(PermissionAnswer::AllowConversation) => {
                    allow_for_conversation(turn, context, &call.name).await;
                }
                Some(PermissionAnswer::AllowOnce) => {}
            }
        }
        Decision::Run => {}
    }
    let status = tools::status_text(call);
    send(ChatEvent::ToolStart {
        call: call.clone(),
        status: status.clone(),
    });
    send(ChatEvent::Status {
        phase: Phase::Work,
        text: Some(status),
    });
    let output = tools::run(env, call, &turn.cancel).await;
    send(ChatEvent::ToolEnd {
        call_id: call.id.clone(),
        ok: !output.is_error,
        summary: output.summary.clone(),
    });
    output
}

/// Sends the request and waits for the answer; `None` when the turn is cancelled first.
async fn ask(
    turn: &Turn,
    send: &mut (dyn FnMut(ChatEvent) + Send),
    mut request: PermissionRequest,
) -> Option<PermissionAnswer> {
    let (id, answer) = turn
        .gate
        .open(&turn.conversation_id, request.dangerous.is_some());
    request.id = id;
    send(ChatEvent::Status {
        phase: Phase::Wait,
        text: Some(tools::status_text(&ToolCall {
            id: request.call_id.clone(),
            name: request.tool.clone(),
            input: request.input.clone(),
        })),
    });
    send(ChatEvent::Permission { request });
    tokio::select! {
        biased;
        () = turn.cancel.cancelled() => {
            turn.gate.close(id);
            None
        }
        answer = answer => answer.ok(),
    }
}

async fn allow_for_conversation(turn: &Turn, context: &mut CallContext<'_>, tool: &str) {
    if !context.allowed.iter().any(|name| name == tool) {
        context.allowed.push(tool.to_owned());
    }
    let (store, workspace, id, tool) = (
        turn.store.clone(),
        turn.workspace.clone(),
        turn.conversation_id.clone(),
        tool.to_owned(),
    );
    let saved = run_blocking(move || {
        store.update_agent(workspace.as_deref(), &id, |agent| {
            if !agent.allowed_tools.contains(&tool) {
                agent.allowed_tools.push(tool);
            }
        })
    })
    .await;
    if let Err(err) = saved {
        tracing::warn!("could not remember the allowed tool: {err}");
    }
}

fn denied(reason: Option<String>) -> ToolOutput {
    let reason = reason
        .map(|r| r.trim().to_owned())
        .filter(|r| !r.is_empty());
    let message = match reason {
        Some(reason) => format!("The user denied this tool call. Their reason: {reason}"),
        None => "The user denied this tool call.".to_owned(),
    };
    ToolOutput {
        content: message,
        is_error: true,
        summary: "denied".into(),
    }
}

fn cancelled() -> ToolOutput {
    ToolOutput {
        content: "Not run: the user stopped the turn.".into(),
        is_error: true,
        summary: "cancelled".into(),
    }
}
