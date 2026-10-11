//! Small synchronization helpers.

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// Locks a mutex, recovering the data if a panicking thread poisoned it.
/// The guarded state stays usable: a poisoned lock never takes the app down.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A one-shot flag other threads can wait on with a timeout.
#[derive(Debug, Default)]
pub struct Signal {
    set: Mutex<bool>,
    changed: Condvar,
}

impl Signal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self) {
        *lock(&self.set) = true;
        self.changed.notify_all();
    }

    pub fn is_set(&self) -> bool {
        *lock(&self.set)
    }

    /// Returns `true` if the signal was set before `timeout` elapsed.
    pub fn wait_timeout(&self, timeout: Duration) -> bool {
        let guard = lock(&self.set);
        let (guard, _) = self
            .changed
            .wait_timeout_while(guard, timeout, |set| !*set)
            .unwrap_or_else(PoisonError::into_inner);
        *guard
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn signal_wakes_waiters() {
        let signal = Arc::new(Signal::new());
        assert!(!signal.wait_timeout(Duration::from_millis(5)));
        let setter = Arc::clone(&signal);
        let handle = std::thread::spawn(move || setter.set());
        assert!(signal.wait_timeout(Duration::from_secs(5)));
        assert!(signal.is_set());
        handle.join().expect("join");
    }
}
