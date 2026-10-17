//! Turns a stored conversation into a provider request: attachments become fenced blocks (a
//! file attached again later is sent once, its latest version; earlier copies become a short
//! reference), failed replies are left out, a Compact summary replaces the messages it
//! covers, and turns alternate starting (and ending) with the user.

use std::collections::HashMap;

use super::cache::{place_breakpoints, BlockRef};
use crate::conversations::{Attachment, ChatMessage, Conversation, MessageStatus, Role};
use crate::error::{AppError, AppResult};
use crate::providers::{ChatRequest, ChatRole, ChatTurn, TurnBlock};
use crate::settings::ProviderEntry;

/// `caching`: Anthropic prompt caching (breakpoints from `cache.rs`).
pub fn build_request(
    conversation: &Conversation,
    entry: &ProviderEntry,
    caching: bool,
) -> AppResult<ChatRequest> {
    let messages = requested_messages(conversation);
    let latest = latest_attachments(&messages);
    let mut turns: Vec<ChatTurn> = Vec::new();
    // Per turn: the indices of its attachment blocks (cache breakpoint candidates).
    let mut attached: Vec<Vec<usize>> = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        let (role, mut blocks, files) = match message.role {
            Role::User => (
                ChatRole::User,
                user_blocks(index, message, &latest),
                message.attachments.len(),
            ),
            Role::Assistant => (
                ChatRole::Assistant,
                vec![TurnBlock::text(message.content.clone())],
                0,
            ),
        };
        match (turns.last_mut(), attached.last_mut()) {
            // Providers want alternating roles: merge neighbours with the same role.
            (Some(last), Some(files_of_last)) if last.role == role => {
                if let Some(first) = blocks.first_mut() {
                    first.text.insert_str(
                        0, "

",
                    );
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
            "Summary of the earlier part of this conversation:

{}

",
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
    if turns.last().is_none_or(|t| t.role != ChatRole::User) {
        return Err(AppError::InvalidInput("nothing to send".into()));
    }
    let attachment_blocks: Vec<BlockRef> = attached
        .iter()
        .enumerate()
        .flat_map(|(turn, blocks)| blocks.iter().map(move |&block| BlockRef { turn, block }))
        .collect();
    let mut chat = ChatRequest {
        model: conversation.model.clone(),
        system: conversation.system_prompt.trim().to_owned(),
        turns,
        max_tokens: entry.max_tokens,
        temperature: entry.temperature,
        caching,
    };
    if caching {
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
        Role::User => true,
        Role::Assistant => {
            message.status != MessageStatus::Error && !message.content.trim().is_empty()
        }
    }
}

/// For each attached path: (message index, attachment index) of its last copy.
fn latest_attachments<'a>(messages: &[&'a ChatMessage]) -> HashMap<&'a str, (usize, usize)> {
    let mut latest = HashMap::new();
    for (m, message) in messages.iter().enumerate() {
        for (a, attachment) in message.attachments.iter().enumerate() {
            latest.insert(attachment.path.as_str(), (m, a));
        }
    }
    latest
}

fn user_blocks(
    index: usize,
    message: &ChatMessage,
    latest: &HashMap<&str, (usize, usize)>,
) -> Vec<TurnBlock> {
    let mut blocks: Vec<TurnBlock> = message
        .attachments
        .iter()
        .enumerate()
        .map(|(a, attachment)| {
            let text = if latest.get(attachment.path.as_str()) == Some(&(index, a)) {
                fenced(attachment)
            } else {
                format!(
                    "File: {} (a newer copy is attached later in this conversation)\n\n",
                    attachment.path
                )
            };
            TurnBlock::text(text)
        })
        .collect();
    blocks.push(TurnBlock::text(message.content.clone()));
    blocks
}

/// The attached files (each in a fence with its relative path), then the prompt.
pub fn user_content(text: &str, attachments: &[Attachment]) -> String {
    let mut out: String = attachments.iter().map(fenced).collect();
    out.push_str(text);
    out
}

fn fenced(attachment: &Attachment) -> String {
    let fence = "`".repeat(longest_backtick_run(&attachment.content).max(2) + 1);
    let lang = attachment
        .path
        .rsplit_once('.')
        .map(|(_, ext)| ext)
        .filter(|ext| !ext.contains('/') && ext.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("");
    let mut out = format!("File: {}\n{fence}{lang}\n", attachment.path);
    out.push_str(&attachment.content);
    if !attachment.content.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&fence);
    out.push_str("\n\n");
    out
}

fn longest_backtick_run(text: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    for c in text.chars() {
        run = if c == '`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    longest
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
