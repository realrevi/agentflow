//! Managed state: one JSON file per conversation under `<app data>/conversations/<workspace>/`,
//! written atomically. Writes are serialized so a reply and a rename never lose each other.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use serde::de::IgnoredAny;
use serde::Deserialize;

use super::checks::{
    auto_title, check_model, check_prompt, check_provider, check_system_prompt, clip, collapse,
    is_valid_id, new_id, MAX_TITLE_CHARS,
};
use super::model::{
    Attachment, ChatMessage, Compaction, Conversation, ConversationPatch, ConversationSummary,
    MessageStatus, Role, FORMAT_VERSION,
};
use crate::error::{AppError, AppResult};
use crate::util::{lock, now_ms, workspace_key, write_atomic};

const TRASH_DIR: &str = ".trash";
const TRASH_KEEP: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Cheap to clone (commands move a copy onto the blocking pool); clones share the lock.
#[derive(Clone)]
pub struct ConversationStore {
    root: PathBuf,
    writes: Arc<Mutex<()>>,
}

impl ConversationStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            writes: Arc::new(Mutex::new(())),
        }
    }

    /// Most recently updated first. Unreadable files are skipped (and logged).
    pub fn list(&self, workspace: Option<&str>) -> AppResult<Vec<ConversationSummary>> {
        let dir = self.dir(workspace);
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err.into()),
        };
        let mut out: Vec<ConversationSummary> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .filter_map(|path| read_summary(&path))
            .collect();
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(a.id.cmp(&b.id)));
        Ok(out)
    }

    pub fn get(&self, workspace: Option<&str>, id: &str) -> AppResult<Conversation> {
        let path = self.file(workspace, id)?;
        let bytes = fs::read(&path).map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => AppError::NotFound(format!("conversation {id}")),
            _ => err.into(),
        })?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn create(
        &self,
        workspace: Option<&str>,
        provider_id: Option<String>,
        model: &str,
        system_prompt: &str,
    ) -> AppResult<Conversation> {
        let _guard = lock(&self.writes);
        let now = now_ms();
        let conversation = Conversation {
            version: FORMAT_VERSION,
            id: new_id('c'),
            workspace: workspace.map(str::to_owned),
            provider_id: check_provider(provider_id)?,
            model: check_model(model)?,
            system_prompt: check_system_prompt(system_prompt)?,
            created_at: now,
            updated_at: now,
            ..Conversation::default()
        };
        self.save(workspace, &conversation)?;
        Ok(conversation)
    }

    pub fn update(
        &self,
        workspace: Option<&str>,
        id: &str,
        patch: ConversationPatch,
    ) -> AppResult<ConversationSummary> {
        self.modify(workspace, id, |conversation| {
            if let Some(title) = patch.title {
                conversation.title = clip(collapse(&title).as_str(), MAX_TITLE_CHARS);
            }
            if let Some(provider_id) = patch.provider_id {
                conversation.provider_id = check_provider(provider_id)?;
            }
            if let Some(model) = patch.model {
                conversation.model = check_model(&model)?;
            }
            if let Some(system_prompt) = patch.system_prompt {
                conversation.system_prompt = check_system_prompt(&system_prompt)?;
            }
            Ok(())
        })
        .map(|conversation| ConversationSummary::from(&conversation))
    }

    /// Moves the file to the app's trash folder (kept 30 days), never deleting outright.
    pub fn delete(&self, workspace: Option<&str>, id: &str) -> AppResult<()> {
        let _guard = lock(&self.writes);
        let path = self.file(workspace, id)?;
        if !path.is_file() {
            return Err(AppError::NotFound(format!("conversation {id}")));
        }
        let trash = self.root.join(TRASH_DIR).join(workspace_key(workspace));
        fs::create_dir_all(&trash)?;
        fs::rename(&path, trash.join(format!("{id}-{}.json", now_ms())))?;
        prune_trash(&self.root.join(TRASH_DIR));
        Ok(())
    }

    /// Stores a new user message; the first one also names the conversation.
    pub fn append_user(
        &self,
        workspace: Option<&str>,
        id: &str,
        text: &str,
        attachments: Vec<Attachment>,
    ) -> AppResult<(Conversation, ChatMessage)> {
        check_prompt(text, &attachments)?;
        let message = ChatMessage {
            id: new_id('m'),
            role: Role::User,
            content: text.to_owned(),
            attachments,
            at: now_ms(),
            status: MessageStatus::Complete,
            stop_reason: None,
            error: None,
            usage: None,
        };
        let stored = message.clone();
        let conversation = self.modify(workspace, id, move |conversation| {
            if conversation.title.is_empty() {
                conversation.title = auto_title(&message.content, &message.attachments);
            }
            conversation.messages.push(message);
            Ok(())
        })?;
        Ok((conversation, stored))
    }

    /// For "Retry": drops failed replies at the end, which must leave a user message last.
    pub fn prepare_retry(&self, workspace: Option<&str>, id: &str) -> AppResult<Conversation> {
        self.modify(workspace, id, |conversation| {
            while conversation
                .messages
                .last()
                .is_some_and(|m| m.role == Role::Assistant && m.status == MessageStatus::Error)
            {
                conversation.messages.pop();
            }
            match conversation.messages.last() {
                Some(last) if last.role == Role::User => Ok(()),
                _ => Err(AppError::InvalidInput("nothing to retry".into())),
            }
        })
    }

    pub fn append_assistant(
        &self,
        workspace: Option<&str>,
        id: &str,
        message: ChatMessage,
    ) -> AppResult<()> {
        self.modify(workspace, id, move |conversation| {
            conversation.messages.push(message);
            Ok(())
        })
        .map(|_| ())
    }

    /// Stores a Compact summary; the message it ends at must still exist.
    pub fn set_compaction(
        &self,
        workspace: Option<&str>,
        id: &str,
        compaction: Compaction,
    ) -> AppResult<Conversation> {
        self.modify(workspace, id, move |conversation| {
            if !conversation
                .messages
                .iter()
                .any(|m| m.id == compaction.through)
            {
                return Err(AppError::InvalidInput(
                    "the chat changed; compact again".into(),
                ));
            }
            conversation.compaction = Some(compaction);
            Ok(())
        })
    }

    fn modify(
        &self,
        workspace: Option<&str>,
        id: &str,
        change: impl FnOnce(&mut Conversation) -> AppResult<()>,
    ) -> AppResult<Conversation> {
        let _guard = lock(&self.writes);
        let mut conversation = self.get(workspace, id)?;
        change(&mut conversation)?;
        conversation.updated_at = now_ms().max(conversation.updated_at);
        self.save(workspace, &conversation)?;
        Ok(conversation)
    }

    fn save(&self, workspace: Option<&str>, conversation: &Conversation) -> AppResult<()> {
        let path = self.file(workspace, &conversation.id)?;
        write_atomic(&path, &serde_json::to_vec_pretty(conversation)?)?;
        Ok(())
    }

    fn dir(&self, workspace: Option<&str>) -> PathBuf {
        self.root.join(workspace_key(workspace))
    }

    fn file(&self, workspace: Option<&str>, id: &str) -> AppResult<PathBuf> {
        if !is_valid_id(id) {
            return Err(AppError::InvalidInput(format!("bad conversation id: {id}")));
        }
        Ok(self.dir(workspace).join(format!("{id}.json")))
    }
}

/// Reads only what the list needs; message bodies are skipped, not kept in memory.
fn read_summary(path: &Path) -> Option<ConversationSummary> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Head {
        id: String,
        #[serde(default)]
        title: String,
        #[serde(default)]
        provider_id: Option<String>,
        #[serde(default)]
        model: String,
        #[serde(default)]
        created_at: u64,
        #[serde(default)]
        updated_at: u64,
        #[serde(default)]
        messages: Vec<IgnoredAny>,
    }
    let parsed = fs::read(path)
        .map_err(|err| err.to_string())
        .and_then(|bytes| serde_json::from_slice::<Head>(&bytes).map_err(|err| err.to_string()));
    match parsed {
        Ok(head) => Some(ConversationSummary {
            id: head.id,
            title: head.title,
            provider_id: head.provider_id,
            model: head.model,
            created_at: head.created_at,
            updated_at: head.updated_at,
            message_count: head.messages.len(),
        }),
        Err(err) => {
            tracing::warn!("skipping unreadable conversation {}: {err}", path.display());
            None
        }
    }
}

fn prune_trash(trash: &Path) {
    let Some(cutoff) = SystemTime::now().checked_sub(TRASH_KEEP) else {
        return;
    };
    let Ok(workspaces) = fs::read_dir(trash) else {
        return;
    };
    for file in workspaces
        .filter_map(Result::ok)
        .filter_map(|dir| fs::read_dir(dir.path()).ok())
        .flatten()
        .filter_map(Result::ok)
    {
        let old = file
            .metadata()
            .and_then(|meta| meta.modified())
            .is_ok_and(|modified| modified < cutoff);
        if old {
            let _ = fs::remove_file(file.path());
        }
    }
}
