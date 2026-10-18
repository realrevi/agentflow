//! The agent loop of one user prompt: model call -> tool calls (permission, run) -> results
//! -> next model call, until the model stops, the iteration limit is hit or the user cancels.

use std::path::Path;
use std::time::Duration;

use super::capabilities::{rejects_tools, ModelCapabilities};
use super::checkpoint::CheckpointStore;
use super::execute::{execute, CallContext};
use super::permissions::PermissionGate;
use super::prompt;
use super::step::{model_call, store_message};
use super::tools::{self, ToolEnv};
use crate::chat::{ChatEvent, Phase};
use crate::conversations::{ChatMessage, ConversationStore, MessageStatus, Role};
use crate::fs::WorkspaceRoot;
use crate::providers::{CancelToken, Http, Part, Provider, ProviderError, StreamEnd, ToolSpec};
use crate::pty::default_shell;
use crate::settings::{ProviderEntry, Settings};
use crate::tokens::UsageTracker;
use crate::util::{now_ms, run_blocking};

/// Everything one turn needs.
pub struct Turn {
    pub http: Http,
    pub provider: Box<dyn Provider>,
    pub entry: ProviderEntry,
    /// Only to redact it from error messages.
    pub secret: Option<String>,
    pub workspace: Option<String>,
    pub conversation_id: String,
    pub panel: Option<String>,
    /// The id of the user message that started the turn (the checkpoint key).
    pub turn_id: String,
    /// Appended to the user's system prompt (workstream manager instructions).
    pub extra_system: Option<String>,
    pub settings: Settings,
    pub store: ConversationStore,
    pub tracker: UsageTracker,
    pub gate: PermissionGate,
    pub checkpoints: CheckpointStore,
    pub capabilities: ModelCapabilities,
    pub cancel: CancelToken,
}

pub async fn run_turn<F>(turn: Turn, mut send: F)
where
    F: FnMut(ChatEvent) + Send,
{
    let send: &mut (dyn FnMut(ChatEvent) + Send) = &mut send;
    if let Err(err) = run(&turn, send).await {
        send(ChatEvent::Error {
            error: ProviderError::config(err.to_string()),
            message: None,
        });
    }
}

async fn run(
    turn: &Turn,
    send: &mut (dyn FnMut(ChatEvent) + Send),
) -> Result<(), crate::error::AppError> {
    let conversation = {
        let (store, workspace, id) = (
            turn.store.clone(),
            turn.workspace.clone(),
            turn.conversation_id.clone(),
        );
        run_blocking(move || store.get(workspace.as_deref(), &id)).await?
    };
    let agent = &turn.settings.agent;
    let root = turn
        .workspace
        .as_deref()
        .and_then(|w| WorkspaceRoot::open(Path::new(w)).ok());
    let mode = conversation
        .agent
        .permission_mode
        .unwrap_or(agent.default_mode);
    let tools_on = root.is_some()
        && !conversation.agent.tools_disabled
        && turn
            .capabilities
            .supports_tools(agent, &turn.entry.id, &conversation.model);
    let profile = conversation.agent.profile.clone();
    // An agent profile's tool list only narrows what the mode offers.
    let mut offered: Vec<ToolSpec> = if tools_on {
        let mut specs = tools::specs(mode);
        specs.retain(|spec| profile.as_ref().is_none_or(|p| p.allows(&spec.name)));
        specs
    } else {
        Vec::new()
    };
    let environment = environment(turn, &conversation.agent.context, root.as_ref()).await;
    let env = root.map(|root| {
        let checkpoint = turn.checkpoints.turn(
            turn.workspace.as_deref(),
            &turn.conversation_id,
            &turn.turn_id,
            root.path(),
        );
        ToolEnv {
            root,
            cap: agent.tool_result_cap as usize,
            timeout: Duration::from_secs(u64::from(agent.command_timeout_secs)),
            checkpoint,
        }
    });
    let mut allowed = conversation.agent.allowed_tools.clone();
    let user_prompt = match &turn.extra_system {
        Some(extra) => format!("{}\n\n{extra}", conversation.system_prompt.trim()),
        None => conversation.system_prompt.clone(),
    };
    for _ in 0..agent.max_iterations {
        let system = prompt::system_prompt(
            &environment,
            &offered,
            mode,
            &user_prompt,
            agent.default_prompt,
            profile.as_ref(),
        );
        send(ChatEvent::Status {
            phase: Phase::Think,
        });
        let step = model_call(turn, send, system, &offered).await?;
        if let Err(err) = &step.outcome {
            if !offered.is_empty() && step.empty && rejects_tools(err) {
                tools_unsupported(turn, &conversation.model).await;
                offered.clear();
                send(ChatEvent::Notice {
                    kind: "toolsUnsupported".into(),
                    message: err.message.clone(),
                });
                continue;
            }
        }
        match step.outcome {
            Err(error) => {
                send(ChatEvent::Error {
                    error,
                    message: step.message,
                });
                return Ok(());
            }
            Ok(StreamEnd::Cancelled) => {
                let closed =
                    close_calls(turn, &step.calls, "Not run: the user stopped the turn.").await;
                if let Some(message) = closed {
                    // The step's text and calls, then the results that close them.
                    if let Some(step) = step.message {
                        send(ChatEvent::Message { message: step });
                    }
                    send(stop("cancelled", Some(message)));
                } else {
                    send(stop("cancelled", step.message));
                }
                return Ok(());
            }
            Ok(StreamEnd::Stopped(reason)) if step.calls.is_empty() => {
                send(stop(&reason, step.message));
                return Ok(());
            }
            Ok(StreamEnd::Stopped(_)) => {
                if let Some(message) = step.message {
                    send(ChatEvent::Message { message });
                }
            }
        }
        let mut context = CallContext {
            env: env.as_ref(),
            mode,
            allowed: &mut allowed,
            profile: profile.as_ref(),
        };
        let mut results = Vec::with_capacity(step.calls.len());
        for call in &step.calls {
            results.push(execute(turn, send, &mut context, call).await);
        }
        let message = tool_message(results);
        store_message(turn, message.clone()).await;
        send(ChatEvent::Message { message });
        if turn.cancel.is_cancelled() {
            send(stop("cancelled", None));
            return Ok(());
        }
    }
    send(ChatEvent::Notice {
        kind: "maxIterations".into(),
        message: format!(
            "Stopped after {} steps (the limit in Settings › Agents). Send a message to continue.",
            agent.max_iterations
        ),
    });
    send(stop("max_iterations", None));
    Ok(())
}

fn stop(reason: &str, message: Option<ChatMessage>) -> ChatEvent {
    ChatEvent::Stop {
        reason: reason.to_owned(),
        message,
    }
}

/// The environment part of the system prompt: stored with the conversation the first time,
/// so later requests reuse the same bytes.
async fn environment(turn: &Turn, stored: &Option<String>, root: Option<&WorkspaceRoot>) -> String {
    if let Some(stored) = stored {
        return stored.clone();
    }
    let environment = prompt::environment(root.map(WorkspaceRoot::path), &default_shell().name());
    if turn.settings.agent.default_prompt {
        let (store, workspace, id, text) = (
            turn.store.clone(),
            turn.workspace.clone(),
            turn.conversation_id.clone(),
            environment.clone(),
        );
        let saved = run_blocking(move || {
            store.update_agent(workspace.as_deref(), &id, |agent| {
                agent.context = Some(text)
            })
        })
        .await;
        if let Err(err) = saved {
            tracing::warn!("could not store the agent environment: {err}");
        }
    }
    environment
}

/// The provider rejected tools for this model: this chat continues without them, and the
/// model is remembered as tools-off.
async fn tools_unsupported(turn: &Turn, model: &str) {
    turn.capabilities
        .remember_unsupported(&turn.entry.id, model);
    let (store, workspace, id) = (
        turn.store.clone(),
        turn.workspace.clone(),
        turn.conversation_id.clone(),
    );
    let saved = run_blocking(move || {
        store.update_agent(workspace.as_deref(), &id, |agent| {
            agent.tools_disabled = true
        })
    })
    .await;
    if let Err(err) = saved {
        tracing::warn!("could not store the tools-off flag: {err}");
    }
}

fn tool_message(results: Vec<crate::providers::ToolResult>) -> ChatMessage {
    let summary = format!("{} tool result(s)", results.len());
    ChatMessage {
        id: format!("t{:x}", now_ms()),
        role: Role::Tool,
        content: summary,
        attachments: Vec::new(),
        at: now_ms(),
        status: MessageStatus::Complete,
        stop_reason: None,
        error: None,
        usage: None,
        parts: results.into_iter().map(Part::ToolResult).collect(),
        prefix: None,
    }
}

/// Calls that will never run (the turn was cancelled) get a result, so the history stays
/// valid for the next request.
async fn close_calls(
    turn: &Turn,
    calls: &[crate::providers::ToolCall],
    reason: &str,
) -> Option<ChatMessage> {
    if calls.is_empty() {
        return None;
    }
    let results = calls
        .iter()
        .map(|call| crate::providers::ToolResult {
            call_id: call.id.clone(),
            content: reason.to_owned(),
            is_error: true,
            outcome: crate::providers::ToolOutcome::Skipped,
            ..Default::default()
        })
        .collect();
    let message = tool_message(results);
    store_message(turn, message.clone()).await;
    Some(message)
}
