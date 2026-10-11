//! Output flow control. The reader counts every byte it hands to the frontend; the frontend
//! acknowledges bytes once xterm has parsed them. Past the high-water mark the reader stops
//! reading, the kernel's pty buffer fills up and the child blocks on write, so a flood of output
//! (`yes`, `cat` of a huge file) cannot grow memory in Rust or in the webview without bound.

use std::sync::{Condvar, Mutex, PoisonError};

use crate::util::lock;

/// Unacknowledged bytes allowed in flight (Rust queue + IPC + xterm's write buffer).
pub const HIGH_WATER: usize = 512 * 1024;

#[derive(Debug, Default)]
pub struct Flow {
    state: Mutex<State>,
    changed: Condvar,
}

#[derive(Debug, Default)]
struct State {
    unacked: usize,
    closed: bool,
}

impl Flow {
    pub fn new() -> Self {
        Self::default()
    }

    /// Called by the reader for every chunk it forwards.
    pub fn sent(&self, bytes: usize) {
        let mut state = lock(&self.state);
        state.unacked = state.unacked.saturating_add(bytes);
    }

    /// Called when the frontend reports bytes as processed.
    pub fn ack(&self, bytes: usize) {
        let mut state = lock(&self.state);
        state.unacked = state.unacked.saturating_sub(bytes);
        self.changed.notify_all();
    }

    /// Wakes a waiting reader for good (the session is gone).
    pub fn close(&self) {
        lock(&self.state).closed = true;
        self.changed.notify_all();
    }

    /// Blocks while too much output is unacknowledged. Returns `false` once closed.
    pub fn wait_for_capacity(&self) -> bool {
        let state = lock(&self.state);
        let state = self
            .changed
            .wait_while(state, |s| s.unacked >= HIGH_WATER && !s.closed)
            .unwrap_or_else(PoisonError::into_inner);
        !state.closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn blocks_above_the_high_water_mark_until_acked() {
        let flow = Arc::new(Flow::new());
        flow.sent(HIGH_WATER);
        let waiter = {
            let flow = Arc::clone(&flow);
            std::thread::spawn(move || flow.wait_for_capacity())
        };
        std::thread::sleep(Duration::from_millis(50));
        assert!(!waiter.is_finished(), "reader should be paused");
        flow.ack(1024);
        assert!(waiter.join().expect("join"));
    }

    #[test]
    fn close_releases_a_paused_reader() {
        let flow = Arc::new(Flow::new());
        flow.sent(HIGH_WATER * 2);
        let waiter = {
            let flow = Arc::clone(&flow);
            std::thread::spawn(move || flow.wait_for_capacity())
        };
        std::thread::sleep(Duration::from_millis(20));
        flow.close();
        assert!(!waiter.join().expect("join"));
    }

    #[test]
    fn acks_never_underflow() {
        let flow = Flow::new();
        flow.ack(10);
        flow.sent(5);
        assert!(flow.wait_for_capacity());
    }
}
