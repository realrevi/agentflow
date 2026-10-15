//! Managed state: the replies streaming right now, so Stop can reach them and a chat never
//! streams two replies at once. Window reloads and app exit cancel them all.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::error::{AppError, AppResult};
use crate::providers::{cancel_pair, CancelHandle, CancelToken, Http};
use crate::util::lock;

struct Running {
    conversation: String,
    handle: CancelHandle,
}

#[derive(Clone, Default)]
pub struct ChatManager {
    running: Arc<Mutex<HashMap<u64, Running>>>,
    next_id: Arc<AtomicU64>,
    http: Arc<OnceLock<Result<Http, String>>>,
}

impl ChatManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// The shared HTTP client, created on first use.
    pub fn http(&self) -> AppResult<Http> {
        self.http
            .get_or_init(Http::new)
            .clone()
            .map_err(AppError::Internal)
    }

    #[cfg(test)]
    pub fn with_http(http: Http) -> Self {
        let manager = Self::default();
        let _ = manager.http.set(Ok(http));
        manager
    }

    /// Reserves `conversation` (a workspace-qualified id) for one streaming reply.
    pub fn register(&self, conversation: String) -> AppResult<(u64, CancelToken)> {
        let mut running = lock(&self.running);
        if running.values().any(|r| r.conversation == conversation) {
            return Err(AppError::InvalidInput(
                "a reply is already streaming in this chat".into(),
            ));
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let (handle, token) = cancel_pair();
        running.insert(
            id,
            Running {
                conversation,
                handle,
            },
        );
        Ok((id, token))
    }

    pub fn finish(&self, id: u64) {
        lock(&self.running).remove(&id);
    }

    /// Stops a streaming reply; `false` if it already ended.
    pub fn cancel(&self, id: u64) -> bool {
        match lock(&self.running).get(&id) {
            Some(running) => {
                running.handle.cancel();
                true
            }
            None => false,
        }
    }

    pub fn cancel_all(&self) {
        for running in lock(&self.running).values() {
            running.handle.cancel();
        }
    }
}
