//! Turns a stored conversation into a provider request: attachments become fenced blocks,
//! failed replies are left out, and turns alternate starting (and ending) with the user.

use crate::conversations::{Attachment, ChatMessage, Conversation, MessageStatus, Role};
use crate::error::{AppError, AppResult};
use crate::providers::{ChatRequest, ChatRole, ChatTurn};
use crate::settings::ProviderEntry;

pub fn build_request(conversation: &Conversation, entry: &ProviderEntry) -> AppResult<ChatRequest> {
    let mut turns: Vec<ChatTurn> = Vec::new();
    for message in conversation.messages.iter().filter(|m| include(m)) {
        let role = match message.role {
            Role::User => ChatRole::User,
            Role::Assistant => ChatRole::Assistant,
        };
        let content = match message.role {
            Role::User => user_content(&message.content, &message.attachments),
            Role::Assistant => message.content.clone(),
        };
        match turns.last_mut() {
            // Providers want alternating roles: merge neighbours with the same role.
            Some(last) if last.role == role => {
                last.content.push_str("\n\n");
                last.content.push_str(&content);
            }
            _ => turns.push(ChatTurn { role, content }),
        }
    }
    while turns.first().is_some_and(|t| t.role == ChatRole::Assistant) {
        turns.remove(0);
    }
    if turns.last().is_none_or(|t| t.role != ChatRole::User) {
        return Err(AppError::InvalidInput("nothing to send".into()));
    }
    Ok(ChatRequest {
        model: conversation.model.clone(),
        system: conversation.system_prompt.trim().to_owned(),
        turns,
        max_tokens: entry.max_tokens,
        temperature: entry.temperature,
    })
}

/// Failed replies are dropped; a stopped reply keeps what had arrived (if anything).
fn include(message: &ChatMessage) -> bool {
    match message.role {
        Role::User => true,
        Role::Assistant => {
            message.status != MessageStatus::Error && !message.content.trim().is_empty()
        }
    }
}

/// The attached files (each in a fence with its relative path), then the prompt.
pub fn user_content(text: &str, attachments: &[Attachment]) -> String {
    let mut out = String::new();
    for attachment in attachments {
        let fence = "`".repeat(longest_backtick_run(&attachment.content).max(2) + 1);
        let lang = attachment
            .path
            .rsplit_once('.')
            .map(|(_, ext)| ext)
            .filter(|ext| !ext.contains('/') && ext.chars().all(|c| c.is_ascii_alphanumeric()))
            .unwrap_or("");
        out.push_str(&format!("File: {}\n{fence}{lang}\n", attachment.path));
        out.push_str(&attachment.content);
        if !attachment.content.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&fence);
        out.push_str("\n\n");
    }
    out.push_str(text);
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
mod tests {
    use super::*;

    fn message(role: Role, content: &str, status: MessageStatus) -> ChatMessage {
        ChatMessage {
            id: "m".into(),
            role,
            content: content.into(),
            attachments: Vec::new(),
            at: 0,
            status,
            stop_reason: None,
            error: None,
            usage: None,
        }
    }

    fn conversation(messages: Vec<ChatMessage>) -> Conversation {
        Conversation {
            model: "m1".into(),
            system_prompt: "  Be brief.  ".into(),
            messages,
            ..Conversation::default()
        }
    }

    #[test]
    fn alternates_and_drops_failed_replies() {
        use MessageStatus::*;
        let request = build_request(
            &conversation(vec![
                message(Role::Assistant, "orphan", Complete),
                message(Role::User, "one", Complete),
                message(Role::Assistant, "partial", Error),
                message(Role::User, "two", Complete),
                message(Role::Assistant, "half", Cancelled),
                message(Role::Assistant, "", Cancelled),
                message(Role::User, "three", Complete),
            ]),
            &ProviderEntry::default(),
        )
        .expect("request");
        let turns: Vec<(ChatRole, &str)> = request
            .turns
            .iter()
            .map(|t| (t.role, t.content.as_str()))
            .collect();
        assert_eq!(
            turns,
            vec![
                (ChatRole::User, "one\n\ntwo"),
                (ChatRole::Assistant, "half"),
                (ChatRole::User, "three"),
            ]
        );
        assert_eq!(request.system, "Be brief.");
        assert_eq!(request.max_tokens, ProviderEntry::default().max_tokens);
    }

    #[test]
    fn needs_a_user_message_last() {
        let done = conversation(vec![
            message(Role::User, "q", MessageStatus::Complete),
            message(Role::Assistant, "a", MessageStatus::Complete),
        ]);
        assert!(build_request(&done, &ProviderEntry::default()).is_err());
        assert!(build_request(&conversation(Vec::new()), &ProviderEntry::default()).is_err());
    }

    #[test]
    fn attachments_are_fenced_with_their_path() {
        let files = vec![
            Attachment {
                path: "src/main.rs".into(),
                content: "fn main() {}".into(),
            },
            Attachment {
                path: "docs/README".into(),
                content: "use ```code``` here\n".into(),
            },
        ];
        let content = user_content("Explain", &files);
        assert_eq!(
            content,
            "File: src/main.rs\n```rs\nfn main() {}\n```\n\n\
             File: docs/README\n````\nuse ```code``` here\n````\n\nExplain"
        );
    }
}
