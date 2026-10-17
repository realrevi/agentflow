//! Attachments in requests: each file in a fence with its path; a file attached again later
//! in the conversation is sent once (its latest version), earlier copies become a reference.

use std::collections::HashMap;

use crate::conversations::{Attachment, ChatMessage};
use crate::providers::TurnBlock;

/// For each attached path: (message index, attachment index) of its last copy.
pub fn latest_attachments<'a>(messages: &[&'a ChatMessage]) -> HashMap<&'a str, (usize, usize)> {
    let mut latest = HashMap::new();
    for (m, message) in messages.iter().enumerate() {
        for (a, attachment) in message.attachments.iter().enumerate() {
            latest.insert(attachment.path.as_str(), (m, a));
        }
    }
    latest
}

/// A user message as blocks: its attachments (or references to newer copies), then the text.
pub fn user_blocks(
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

pub fn fenced(attachment: &Attachment) -> String {
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
