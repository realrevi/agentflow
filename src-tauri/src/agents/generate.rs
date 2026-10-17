//! "Create with AI": one tool-less request asks a configured model for a template as JSON;
//! the reply is parsed and validated, and an invalid reply gets one automatic retry with the
//! validation errors. Every call's usage goes to the ledger (`kind: "installer"`). The draft
//! is only returned: installing it still goes through the normal preview.

use serde::{Deserialize, Serialize};

use super::template::Template;
use super::validate::validate;
use crate::chat::ChatDeps;
use crate::error::{AppError, AppResult};
use crate::providers::{
    build_provider, ChatRequest, ChatRole, ChatTurn, EventSink, StreamEnd, StreamEvent, TokenUsage,
    TurnBlock,
};
use crate::tokens::{Source, UsageRecord};
use crate::util::{now_ms, run_blocking};

const SCHEMA: &str = include_str!("../../../schemas/agent-template.schema.json");
const MAX_ATTEMPTS: usize = 2;
/// Output budget for a template: reasoning models spend much of it thinking, so the usual
/// chat limit (often 4096) cuts the JSON off.
const MIN_OUTPUT_TOKENS: u32 = 16_000;
/// Stop reasons that mean "hit the output limit" (OpenAI-style, Anthropic).
const TRUNCATED: [&str; 2] = ["length", "max_tokens"];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    pub workspace: Option<String>,
    pub provider_id: String,
    pub model: String,
    /// The user's plain-words description of the agent.
    pub description: String,
    /// Language the instructions are written in (`en`, or the workspace's language).
    pub language: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateResult {
    pub template: Template,
    pub attempts: usize,
    pub usage: TokenUsage,
}

fn system(language: &str) -> String {
    format!(
        "You design focused AI coding agents for AgentFlow, which installs them into Claude \
         Code, Codex CLI, Gemini CLI, Cursor and AgentFlow. Reply with ONE JSON object that \
         matches this JSON Schema, and nothing else (no code fences, no comments):\n\n{SCHEMA}\n\n\
         Rules:\n\
         - schema 1, version 1.0.0, author \"Created with AI\", license MIT, language \"{language}\".\n\
         - Write `name`, `description`, question labels and `instructions` in language \"{language}\".\n\
         - `instructions`: the agent's role and method as short Markdown (how to work, rules, \
         what to report). Concrete and token-aware, no filler. Use {{{{questionId}}}} \
         placeholders only for question ids you define, or {{{{workspaceName}}}}.\n\
         - Ask only questions that change the instructions; give sensible defaults; use a \
         `detect` key when a workspace detector can fill the answer.\n\
         - Suggest per-target options: Claude Code tools (Read, Grep, Glob, Edit, Write, Bash), \
         AgentFlow permissionMode and tools (read-only agents: readOnly and read tools only)."
    )
}

/// The template in a model reply: the outermost `{...}` (code fences and chatter around it
/// are ignored), parsed and validated. The error lists what to fix.
pub fn parse_reply(text: &str) -> Result<Template, String> {
    let start = text.find('{').ok_or("the reply contains no JSON object")?;
    let end = text.rfind('}').ok_or("the reply contains no JSON object")?;
    let json = text
        .get(start..=end)
        .ok_or("the reply contains no JSON object")?;
    let template: Template =
        serde_json::from_str(json).map_err(|err| format!("the JSON does not match: {err}"))?;
    let errors = validate(&template);
    if errors.is_empty() {
        Ok(template)
    } else {
        Err(errors.join("; "))
    }
}

#[derive(Default)]
struct Collect {
    text: String,
    usage: TokenUsage,
}

impl EventSink for Collect {
    fn event(&mut self, event: StreamEvent) {
        match event {
            StreamEvent::Delta(text) => self.text.push_str(&text),
            StreamEvent::Usage(usage) => self.usage = usage,
            _ => {}
        }
    }
}

pub async fn generate(deps: ChatDeps, request: GenerateRequest) -> AppResult<GenerateResult> {
    let description = request.description.trim();
    if description.is_empty() {
        return Err(AppError::InvalidInput("describe the agent first".into()));
    }
    let entry = deps
        .settings
        .providers
        .iter()
        .find(|p| p.id == request.provider_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound("that provider".into()))?;
    deps.tracker.check_send(&entry.id, None)?;
    let key = {
        let (vault, id) = (deps.vault.clone(), entry.id.clone());
        run_blocking(move || vault.get(&id)).await?
    };
    let secret = key.as_ref().map(|k| k.expose().to_owned());
    let provider = build_provider(&entry, key)?;
    let http = deps.manager.http()?;
    let mut turns = vec![ChatTurn::user(description.to_owned())];
    let mut total = TokenUsage::default();
    let mut last_error = String::new();
    for attempt in 1..=MAX_ATTEMPTS {
        let chat = ChatRequest {
            model: request.model.clone(),
            system: system(&request.language),
            turns: turns.clone(),
            max_tokens: entry.max_tokens.max(MIN_OUTPUT_TOKENS),
            temperature: None,
            caching: false,
            tools: Vec::new(),
        };
        let (request_id, cancel) = deps.manager.register(format!("installer|{}", now_ms()))?;
        let mut sink = Collect::default();
        let outcome = provider.stream_chat(&http, &chat, &mut sink, &cancel).await;
        deps.manager.finish(request_id);
        record(&deps, &entry, &request, sink.usage, attempt).await;
        total.input_tokens += sink.usage.input_tokens;
        total.output_tokens += sink.usage.output_tokens;
        total.cache_read_tokens += sink.usage.cache_read_tokens;
        total.cache_write_tokens += sink.usage.cache_write_tokens;
        let end =
            outcome.map_err(|err| AppError::Provider(err.redacted(secret.as_deref()).message))?;
        let cut_off =
            matches!(&end, StreamEnd::Stopped(reason) if TRUNCATED.contains(&reason.as_str()));
        let parsed = if cut_off {
            Err("the reply was cut off at the output limit".to_owned())
        } else {
            parse_reply(&sink.text)
        };
        match parsed {
            Ok(template) => {
                return Ok(GenerateResult {
                    template,
                    attempts: attempt,
                    usage: total,
                })
            }
            Err(error) => {
                last_error = error.clone();
                turns.push(ChatTurn {
                    role: ChatRole::Assistant,
                    blocks: vec![TurnBlock::text(sink.text)],
                });
                turns.push(ChatTurn::user(if cut_off {
                    "Your reply was cut off at the output limit. Reply again with a shorter                      template (concise instructions, at most 6 questions), JSON only."
                        .to_owned()
                } else {
                    format!(
                        "That JSON is not valid: {error}. Reply with the corrected JSON object only."
                    )
                }));
            }
        }
    }
    Err(AppError::InvalidInput(format!(
        "the model's template is not valid: {last_error}"
    )))
}

async fn record(
    deps: &ChatDeps,
    entry: &crate::settings::ProviderEntry,
    request: &GenerateRequest,
    usage: TokenUsage,
    attempt: usize,
) {
    if usage.is_empty() {
        return;
    }
    let at = now_ms();
    let record = UsageRecord {
        at,
        source: Source::Api,
        provider: entry.id.clone(),
        preset: entry.preset.clone(),
        model: request.model.clone(),
        workspace: request.workspace.clone(),
        panel: None,
        id: format!("installer:{at}:{attempt}"),
        kind: "installer".into(),
        tokens: usage,
    };
    let tracker = deps.tracker.clone();
    if let Err(err) = run_blocking(move || tracker.record(vec![record])).await {
        tracing::warn!("could not record usage: {err}");
    }
}
