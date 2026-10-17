//! Ledger records from stored API chats: every assistant reply that reported usage (phase 5
//! stored one per reply). Run once when the ledger is new; replies in the app trash count
//! too (the money was spent). Ids are `chat:<conversation>:<message>`, the same as live
//! records, so a reply is never counted twice.

use std::path::Path;

use super::record::{Source, UsageRecord};
use crate::conversations::{self, Conversation};

pub const MARKER: &str = ".backfill";
const MAX_DEPTH: usize = 3;

/// A ledger record for one stored reply.
pub fn chat_record(
    conversation_id: &str,
    message_id: &str,
    workspace: Option<&str>,
    panel: Option<&str>,
    usage: &conversations::UsageRecord,
) -> UsageRecord {
    UsageRecord {
        at: usage.at,
        source: Source::Api,
        provider: usage.provider_id.clone(),
        preset: usage.preset.clone(),
        model: usage.model.clone(),
        workspace: workspace.map(str::to_owned),
        panel: panel.map(str::to_owned),
        id: format!("chat:{conversation_id}:{message_id}"),
        kind: String::new(),
        tokens: usage.tokens,
    }
}

/// Records of every conversation file under `root` (`<app data>/conversations`).
pub fn conversation_records(root: &Path) -> Vec<UsageRecord> {
    let mut files = Vec::new();
    collect_json(root, 0, &mut files);
    let mut out = Vec::new();
    for path in files {
        let parsed = std::fs::read(&path)
            .map_err(|err| err.to_string())
            .and_then(|bytes| {
                serde_json::from_slice::<Conversation>(&bytes).map_err(|err| err.to_string())
            });
        let conversation = match parsed {
            Ok(conversation) => conversation,
            Err(err) => {
                tracing::debug!("backfill: skipping {}: {err}", path.display());
                continue;
            }
        };
        for message in &conversation.messages {
            if let Some(usage) = &message.usage {
                out.push(chat_record(
                    &conversation.id,
                    &message.id,
                    conversation.workspace.as_deref(),
                    None,
                    usage,
                ));
            }
        }
    }
    out
}

fn collect_json(dir: &Path, depth: usize, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        if is_dir && depth < MAX_DEPTH {
            collect_json(&path, depth + 1, out);
        } else if !is_dir && path.extension().is_some_and(|ext| ext == "json") {
            out.push(path);
        }
    }
}
