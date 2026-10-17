//! Turns a stored conversation into a provider request: attachments become fenced blocks
//! (see `attachments.rs`), failed replies are left out, a Compact summary replaces the
//! messages it covers, agent steps keep their parts (reasoning, tool calls, tool results as
//! the user's turn), and turns alternate starting (and ending) with the user.

use super::attachments::{latest_attachments, user_blocks};
use super::cache::{place_breakpoints, BlockRef};
use crate::conversations::{ChatMessage, Conversation, MessageStatus, Role};
use crate::error::{AppError, AppResult};
use crate::providers::{ChatRequest, ChatRole, ChatTurn, Part, ToolResult, ToolSpec, TurnBlock};
use crate::settings::ProviderEntry;

pub use super::attachments::user_content;

/// How a request is built beyond the conversation itself.
#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    /// Anthropic prompt caching (breakpoints from `cache.rs`).
    pub caching: bool,
    /// The system prompt; `None` = the conversation's own.
    pub system: Option<String>,
    pub tools: Vec<ToolSpec>,
    /// Anthropic: thinking blocks are replayed only from messages produced under this
    /// prefix digest (a changed system prompt or tool set invalidates them).
    pub prefix: Option<String>,
    pub anthropic: bool,
}

/// A plain chat request (no tools).
pub fn build_request(
    conversation: &Conversation,
    entry: &ProviderEntry,
    caching: bool,
) -> AppResult<ChatRequest> {
    let options = RequestOptions {
        caching,
        anthropic: entry.kind == "anthropic",
        ..RequestOptions::default()
    };
    build_with(conversation, entry, &options)
}

pub fn build_with(
    conversation: &Conversation,
    entry: &ProviderEntry,
    options: &RequestOptions,
) -> AppResult<ChatRequest> {
    let messages = requested_messages(conversation);
    let latest = latest_attachments(&messages);
    let compacted_at = conversation.compaction.as_ref().map(|c| c.at);
    let mut turns: Vec<ChatTurn> = Vec::new();
    // Per turn: the indices of its attachment blocks (cache breakpoint candidates).
    let mut attached: Vec<Vec<usize>> = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        let files = match message.role {
            Role::User => message.attachments.len(),
            _ => 0,
        };
        let (role, mut blocks) = match message.role {
            Role::User => (ChatRole::User, user_blocks(index, message, &latest)),
            Role::Assistant => {
                let replay = replays_thinking(message, options, compacted_at);
                (ChatRole::Assistant, assistant_blocks(message, replay))
            }
            Role::Tool => (ChatRole::User, part_blocks(message, false)),
        };
        match (turns.last_mut(), attached.last_mut()) {
            // Providers want alternating roles: merge neighbours with the same role.
            (Some(last), Some(files_of_last)) if last.role == role => {
                let after_text = last
                    .blocks
                    .last()
                    .is_some_and(|b| matches!(b.part, Part::Text { .. }));
                if let Some(text) = blocks.first_mut().and_then(TurnBlock::text_mut) {
                    if after_text {
                        text.insert_str(0, "\n\n");
                    }
                }
                let start = last.blocks.len();
                files_of_last.extend(start..start + files);
                last.blocks.append(&mut blocks);
            }
            _ => {
                attached.push((0..files).collect());
                turns.push(ChatTurn { role, blocks });
            }
        }
    }
    if let Some(compaction) = &conversation.compaction {
        let summary = TurnBlock::text(format!(
            "Summary of the earlier part of this conversation:\n\n{}\n\n",
            compaction.summary.trim()
        ));
        match turns.first_mut() {
            Some(first) if first.role == ChatRole::User => {
                first.blocks.insert(0, summary);
                attached[0].iter_mut().for_each(|block| *block += 1);
            }
            _ => {
                turns.insert(
                    0,
                    ChatTurn {
                        role: ChatRole::User,
                        blocks: vec![summary],
                    },
                );
                attached.insert(0, Vec::new());
            }
        }
    }
    while turns.first().is_some_and(|t| t.role == ChatRole::Assistant) {
        turns.remove(0);
        attached.remove(0);
    }
    close_dangling_calls(&mut turns, &mut attached);
    if turns.last().is_none_or(|t| t.role != ChatRole::User) {
        return Err(AppError::InvalidInput("nothing to send".into()));
    }
    let attachment_blocks: Vec<BlockRef> = attached
        .iter()
        .enumerate()
        .flat_map(|(turn, blocks)| blocks.iter().map(move |&block| BlockRef { turn, block }))
        .collect();
    let system = match &options.system {
        Some(system) => system.trim().to_owned(),
        None => conversation.system_prompt.trim().to_owned(),
    };
    let mut chat = ChatRequest {
        model: conversation.model.clone(),
        system,
        turns,
        max_tokens: entry.max_tokens,
        temperature: entry.temperature,
        caching: options.caching,
        tools: options.tools.clone(),
    };
    if options.caching {
        place_breakpoints(&mut chat, &attachment_blocks);
    }
    Ok(chat)
}

/// The messages a request sends: after the Compact point, without failed replies.
pub fn requested_messages(conversation: &Conversation) -> Vec<&ChatMessage> {
    let start = conversation
        .compaction
        .as_ref()
        .and_then(|c| conversation.messages.iter().position(|m| m.id == c.through))
        .map_or(0, |i| i + 1);
    conversation.messages[start..]
        .iter()
        .filter(|m| include(m))
        .collect()
}

/// Failed replies are dropped; a stopped reply keeps what had arrived (if anything).
pub fn include(message: &ChatMessage) -> bool {
    match message.role {
        Role::User | Role::Tool => true,
        Role::Assistant => {
            message.status != MessageStatus::Error
                && (!message.content.trim().is_empty() || !message.parts.is_empty())
        }
    }
}

/// Anthropic thinking is valid only under the prefix (system prompt + tools) it was
/// produced under, and not across a Compact (the history before it changed). Other
/// providers get their reasoning back always (DeepSeek requires it once tools are used).
fn replays_thinking(
    message: &ChatMessage,
    options: &RequestOptions,
    compacted: Option<u64>,
) -> bool {
    if !options.anthropic {
        return true;
    }
    let same_prefix = message.prefix.is_some() && message.prefix == options.prefix;
    same_prefix && compacted.is_none_or(|at| message.at > at)
}

fn assistant_blocks(message: &ChatMessage, replay_thinking: bool) -> Vec<TurnBlock> {
    if message.parts.is_empty() {
        return vec![TurnBlock::text(message.content.clone())];
    }
    part_blocks(message, replay_thinking)
}

fn part_blocks(message: &ChatMessage, keep_reasoning: bool) -> Vec<TurnBlock> {
    message
        .parts
        .iter()
        .filter(|part| keep_reasoning || !matches!(part, Part::Reasoning(_)))
        .map(|part| TurnBlock::part(part.clone()))
        .collect()
}

/// Every tool call needs a result in the next (user) turn. A turn that was cancelled or
/// failed before all results were stored gets an error result, so the request stays valid.
fn close_dangling_calls(turns: &mut Vec<ChatTurn>, attached: &mut Vec<Vec<usize>>) {
    let mut index = 0;
    while index < turns.len() {
        if turns[index].role != ChatRole::Assistant {
            index += 1;
            continue;
        }
        let calls: Vec<String> = turns[index]
            .blocks
            .iter()
            .filter_map(|b| match &b.part {
                Part::ToolCall(call) => Some(call.id.clone()),
                _ => None,
            })
            .collect();
        if calls.is_empty() {
            index += 1;
            continue;
        }
        if turns
            .get(index + 1)
            .is_none_or(|t| t.role != ChatRole::User)
        {
            turns.insert(
                index + 1,
                ChatTurn {
                    role: ChatRole::User,
                    blocks: Vec::new(),
                },
            );
            attached.insert(index + 1, Vec::new());
        }
        let next = &mut turns[index + 1];
        let answered: Vec<String> = next
            .blocks
            .iter()
            .filter_map(|b| match &b.part {
                Part::ToolResult(result) => Some(result.call_id.clone()),
                _ => None,
            })
            .collect();
        let missing: Vec<TurnBlock> = calls
            .into_iter()
            .filter(|id| !answered.contains(id))
            .map(|call_id| {
                TurnBlock::part(Part::ToolResult(ToolResult {
                    call_id,
                    content: "No result: the turn was interrupted before this tool ran.".into(),
                    is_error: true,
                }))
            })
            .collect();
        let added = missing.len();
        // Results go first in the user turn (Anthropic requires it).
        next.blocks.splice(0..0, missing);
        attached[index + 1]
            .iter_mut()
            .for_each(|block| *block += added);
        index += 2;
    }
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
