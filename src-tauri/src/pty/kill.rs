//! Forced termination for children that ignore the polite kill. portable-pty's killer
//! sends SIGHUP on Unix (TerminateProcess on Windows, which is already forceful).

use std::sync::Arc;
use std::time::Duration;

use crate::util::Signal;

/// How long a child gets to exit after SIGHUP before it is SIGKILLed.
pub const KILL_GRACE: Duration = Duration::from_secs(2);

/// Waits (on a short-lived thread) for `reaped`; SIGKILLs `pid` if it is still running.
pub fn escalate_after_grace(pid: Option<u32>, reaped: Arc<Signal>) {
    let Some(pid) = pid else { return };
    let spawned = std::thread::Builder::new()
        .name(format!("pty-kill-{pid}"))
        .spawn(move || {
            if !reaped.wait_timeout(KILL_GRACE) {
                force_kill(pid, &reaped);
            }
        });
    if let Err(err) = spawned {
        tracing::warn!("could not start kill watchdog for {pid}: {err}");
    }
}

/// SIGKILL, but only while the child is unreaped: until the waiter thread reaps it, the
/// pid cannot be reused by another process.
#[cfg(unix)]
pub fn force_kill(pid: u32, reaped: &Signal) {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return;
    };
    if reaped.is_set() {
        return;
    }
    tracing::warn!("process {pid} ignored SIGHUP, sending SIGKILL");
    // SAFETY: kill(2) only takes integers and touches no memory of ours.
    unsafe {
        libc::kill(pid, libc::SIGKILL);
    }
}

#[cfg(not(unix))]
pub fn force_kill(_pid: u32, _reaped: &Signal) {}
