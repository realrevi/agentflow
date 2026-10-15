//! Stop button plumbing: a handle the session keeps and a token the stream awaits.

use tokio::sync::watch;

pub struct CancelHandle(watch::Sender<bool>);

#[derive(Clone)]
pub struct CancelToken(watch::Receiver<bool>);

pub fn cancel_pair() -> (CancelHandle, CancelToken) {
    let (tx, rx) = watch::channel(false);
    (CancelHandle(tx), CancelToken(rx))
}

impl CancelHandle {
    pub fn cancel(&self) {
        // send_replace never fails, even when the token is gone.
        self.0.send_replace(true);
    }
}

impl CancelToken {
    pub fn is_cancelled(&self) -> bool {
        *self.0.borrow()
    }

    /// Resolves once cancelled; never resolves if the handle was dropped without cancelling.
    pub async fn cancelled(&self) {
        let mut rx = self.0.clone();
        if rx.wait_for(|cancelled| *cancelled).await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_wakes_the_token() {
        let (handle, token) = cancel_pair();
        assert!(!token.is_cancelled());
        handle.cancel();
        assert!(token.is_cancelled());
        tauri::async_runtime::block_on(token.cancelled());
    }
}
