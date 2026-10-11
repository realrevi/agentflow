//! Validation and naming for conversations: ids, limits, auto titles.

use std::sync::atomic::{AtomicU64, Ordering};

use super::model::Attachment;
use crate::error::{AppError, AppResult};
use crate::settings::is_valid_provider_id;
use crate::util::now_ms;

/// Attachments of one message, in total (bytes of content).
pub const MAX_ATTACHMENT_BYTES: usize = 200 * 1024;
pub const MAX_TITLE_CHARS: usize = 120;
const MAX_PROMPT_BYTES: usize = 512 * 1024;
const MAX_SYSTEM_PROMPT_BYTES: usize = 32 * 1024;
const AUTO_TITLE_CHARS: usize = 60;
const MAX_MODEL_CHARS: usize = 200;
const MAX_ID_LEN: usize = 64;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}

/// Unique within this process, ordered by time, and unlikely to collide across runs.
pub fn new_id(prefix: char) -> String {
    let n = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    format!(
        "{prefix}{:x}{:04x}{:x}",
        now_ms(),
        std::process::id() & 0xffff,
        n
    )
}

pub fn check_prompt(text: &str, attachments: &[Attachment]) -> AppResult<()> {
    if text.trim().is_empty() && attachments.is_empty() {
        return Err(AppError::InvalidInput("empty prompt".into()));
    }
    if text.len() > MAX_PROMPT_BYTES {
        return Err(AppError::InvalidInput("the prompt is too long".into()));
    }
    let total: usize = attachments.iter().map(|a| a.content.len()).sum();
    if total > MAX_ATTACHMENT_BYTES {
        return Err(AppError::TooLarge {
            size: total as u64,
            limit: MAX_ATTACHMENT_BYTES as u64,
        });
    }
    let bad_path = attachments
        .iter()
        .any(|a| a.path.is_empty() || a.path.len() > 1024 || a.path.contains('\0'));
    if bad_path {
        return Err(AppError::InvalidInput("bad attachment path".into()));
    }
    Ok(())
}

pub fn check_provider(provider_id: Option<String>) -> AppResult<Option<String>> {
    match provider_id {
        Some(id) if !is_valid_provider_id(&id) => {
            Err(AppError::InvalidInput(format!("bad provider id: {id}")))
        }
        other => Ok(other),
    }
}

pub fn check_model(model: &str) -> AppResult<String> {
    let model = model.trim();
    if model.chars().count() > MAX_MODEL_CHARS || model.chars().any(char::is_control) {
        return Err(AppError::InvalidInput("bad model id".into()));
    }
    Ok(model.to_owned())
}

pub fn check_system_prompt(prompt: &str) -> AppResult<String> {
    if prompt.len() > MAX_SYSTEM_PROMPT_BYTES {
        return Err(AppError::InvalidInput(
            "the system prompt is too long".into(),
        ));
    }
    Ok(prompt.to_owned())
}

/// The first words of the prompt (or the first attachment's name), on one line.
pub fn auto_title(text: &str, attachments: &[Attachment]) -> String {
    let line = collapse(text);
    let source = if line.is_empty() {
        attachments
            .first()
            .map(|a| a.path.rsplit('/').next().unwrap_or(&a.path).to_owned())
            .unwrap_or_default()
    } else {
        line
    };
    if source.chars().count() > AUTO_TITLE_CHARS {
        format!("{}…", clip(&source, AUTO_TITLE_CHARS - 1))
    } else {
        source
    }
}

pub fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn clip(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}
