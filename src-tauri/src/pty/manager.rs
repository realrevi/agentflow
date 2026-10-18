//! Owns every live pty session, keyed by id. Lives in Tauri state.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, Weak};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, Child, CommandBuilder, PtySize};

use super::coalesce::{run_pump, PumpMsg};
use super::env_path::search_path;
use super::session_env::PARENT_SESSION_VARS;

use super::flow::Flow;
use super::kill::{escalate_after_grace, force_kill};
use super::launch::Launch;
use super::model::{OutputSink, PtyKind, SessionId, SessionInfo, TermSize};
use super::session::{read_loop, wait_loop, Session};
use crate::error::{AppError, AppResult};
use crate::util::{lock, Signal};

/// On app exit: time for children to leave after SIGHUP, then after SIGKILL.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(1);
const SHUTDOWN_FORCE_WAIT: Duration = Duration::from_millis(500);

#[derive(Clone, Default)]
pub struct PtyManager {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    next_id: AtomicU64,
    sessions: Mutex<HashMap<SessionId, Session>>,
    /// Killed sessions whose child may not be reaped yet; shutdown waits for them too.
    dying: Mutex<Vec<Pending>>,
}

/// A child that was told to stop: its pid and the signal set once it is reaped.
type Pending = (Option<u32>, Arc<Signal>);

impl Inner {
    fn remove(&self, id: SessionId) -> Option<Session> {
        lock(&self.sessions).remove(&id)
    }
}

/// What the session threads take over after the pty is open.
struct Threads {
    child: Box<dyn Child + Send + Sync>,
    reader: Box<dyn std::io::Read + Send>,
    reaped: Arc<Signal>,
    flow: Arc<Flow>,
}

fn pty_error(err: impl std::fmt::Display) -> AppError {
    AppError::Pty(err.to_string())
}

impl PtyManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts `launch` in a new pty. Output goes to `sink` until the child exits; the
    /// session then removes itself. Env: inherited, plus TERM and COLORTERM.
    pub fn spawn(
        &self,
        kind: PtyKind,
        launch: &Launch,
        cwd: &Path,
        size: TermSize,
        sink: OutputSink,
    ) -> AppResult<SessionInfo> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(pty_error)?;

        let mut command = CommandBuilder::new(&launch.program);
        command.args(&launch.args);
        command.cwd(cwd);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        if let Some(path) = search_path() {
            command.env("PATH", path); // so agent CLIs also find node etc. on GUI launches
        }
        // A terminal here is a session of its own, even when AgentFlow was started from
        // inside an agent CLI session: its markers must not reach the CLIs run in panels
        // (Claude Code would treat them as its child session and keep no transcript).
        for name in PARENT_SESSION_VARS {
            command.env_remove(name);
        }

        let mut child = pair.slave.spawn_command(command).map_err(pty_error)?;
        // Only the child may hold the slave, so EOF arrives once it (and its children) exit.
        drop(pair.slave);

        let handles = pair
            .master
            .try_clone_reader()
            .and_then(|reader| Ok((reader, pair.master.take_writer()?)));
        let (reader, writer) = match handles {
            Ok(handles) => handles,
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(pty_error(err));
            }
        };

        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let info = SessionInfo {
            id,
            kind,
            program: launch.program.to_string_lossy().into_owned(),
            args: launch.args.clone(),
            cwd: cwd.to_string_lossy().into_owned(),
            pid: child.process_id(),
        };
        let reaped = Arc::new(Signal::new());
        let flow = Arc::new(Flow::new());
        let session = Session {
            info: info.clone(),
            writer: Arc::new(Mutex::new(writer)),
            master: Arc::new(Mutex::new(pair.master)),
            killer: child.clone_killer(),
            reaped: Arc::clone(&reaped),
            flow: Arc::clone(&flow),
        };
        // Insert before the threads start: a fast-exiting child must find itself to remove.
        lock(&self.inner.sessions).insert(id, session);

        let (tx, rx) = mpsc::channel();
        let threads = Threads {
            child,
            reader,
            reaped,
            flow,
        };
        if let Err(err) = self.start_threads(id, threads, (tx, rx), sink) {
            self.kill(id);
            return Err(err);
        }
        tracing::debug!("pty {id} started: {} {:?}", info.program, info.args);
        Ok(info)
    }

    fn start_threads(
        &self,
        id: SessionId,
        threads: Threads,
        (tx, rx): (Sender<PumpMsg>, Receiver<PumpMsg>),
        sink: OutputSink,
    ) -> AppResult<()> {
        let Threads {
            child,
            reader,
            reaped,
            flow,
        } = threads;
        // The waiter starts first: once it runs, the child is always reaped. The child sits in
        // a slot so it can be reaped here if the thread cannot be started at all.
        let child_slot = Arc::new(Mutex::new(Some(child)));
        let waiter_slot = Arc::clone(&child_slot);
        let waiter_tx = tx.clone();
        let waiter = thread::Builder::new()
            .name(format!("pty-{id}-wait"))
            .spawn(move || {
                if let Some(child) = lock(&waiter_slot).take() {
                    wait_loop(child, &reaped, &waiter_tx);
                }
            });
        if let Err(err) = waiter {
            if let Some(mut child) = lock(&child_slot).take() {
                let _ = child.kill();
                let _ = child.wait();
            }
            return Err(err.into());
        }
        let inner: Weak<Inner> = Arc::downgrade(&self.inner);
        thread::Builder::new()
            .name(format!("pty-{id}-pump"))
            .spawn(move || {
                run_pump(&rx, sink); // drops the sink (the Channel) when done
                if let Some(inner) = inner.upgrade() {
                    inner.remove(id);
                }
            })?;
        thread::Builder::new()
            .name(format!("pty-{id}-read"))
            .spawn(move || read_loop(reader, tx, &flow))?;
        Ok(())
    }

    pub fn write(&self, id: SessionId, bytes: &[u8]) -> AppResult<()> {
        let writer = self.with_session(id, |session| Arc::clone(&session.writer))?;
        let mut writer = lock(&writer);
        writer.write_all(bytes)?;
        writer.flush()?;
        Ok(())
    }

    /// The frontend has processed `bytes` more output; lets a paused reader continue.
    pub fn ack(&self, id: SessionId, bytes: usize) -> AppResult<()> {
        let flow = self.with_session(id, |session| Arc::clone(&session.flow))?;
        flow.ack(bytes);
        Ok(())
    }

    pub fn resize(&self, id: SessionId, size: TermSize) -> AppResult<()> {
        let master = self.with_session(id, |session| Arc::clone(&session.master))?;
        let master = lock(&master);
        master
            .resize(PtySize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(pty_error)
    }

    /// Hangs up the session and drops it. Children that ignore the hangup are SIGKILLed
    /// after a grace period; the waiter thread reaps them. Returns `false` if the session
    /// was already gone (e.g. it exited on its own).
    pub fn kill(&self, id: SessionId) -> bool {
        let Some(mut session) = self.inner.remove(id) else {
            return false;
        };
        if let Err(err) = session.killer.kill() {
            tracing::debug!("pty {id}: kill: {err}");
        }
        escalate_after_grace(session.info.pid, Arc::clone(&session.reaped));
        let mut dying = lock(&self.inner.dying);
        dying.retain(|(_, reaped)| !reaped.is_set());
        dying.push((session.info.pid, Arc::clone(&session.reaped)));
        true
    }

    /// Kills every session without waiting (e.g. when the page reloads and every panel's
    /// channel is gone).
    pub fn kill_all(&self) {
        let ids: Vec<SessionId> = lock(&self.inner.sessions).keys().copied().collect();
        for id in ids {
            self.kill(id);
        }
    }

    pub fn list(&self) -> Vec<SessionInfo> {
        let mut infos: Vec<SessionInfo> = lock(&self.inner.sessions)
            .values()
            .map(|session| session.info.clone())
            .collect();
        infos.sort_by_key(|info| info.id);
        infos
    }

    /// Kills every session and waits (bounded) until the children are reaped.
    /// Called on app exit so no process outlives AgentFlow.
    pub fn shutdown(&self) {
        let mut sessions: Vec<Session> = lock(&self.inner.sessions)
            .drain()
            .map(|(_, session)| session)
            .collect();
        for session in &mut sessions {
            let _ = session.killer.kill();
        }
        let mut pending: Vec<Pending> = sessions
            .iter()
            .map(|session| (session.info.pid, Arc::clone(&session.reaped)))
            .collect();
        drop(sessions); // closes every pty master
                        // Sessions killed moments ago may still be inside their SIGKILL grace period.
        pending.extend(lock(&self.inner.dying).drain(..));
        pending.retain(|(_, reaped)| !reaped.is_set());
        if pending.is_empty() {
            return;
        }
        tracing::info!("stopping {} terminal process(es)", pending.len());

        let deadline = Instant::now() + SHUTDOWN_GRACE;
        for (pid, reaped) in &pending {
            let left = deadline.saturating_duration_since(Instant::now());
            if !reaped.wait_timeout(left) {
                if let Some(pid) = pid {
                    force_kill(*pid, reaped);
                }
            }
        }
        let deadline = Instant::now() + SHUTDOWN_FORCE_WAIT;
        for (_, reaped) in &pending {
            reaped.wait_timeout(deadline.saturating_duration_since(Instant::now()));
        }
    }

    fn with_session<T>(&self, id: SessionId, f: impl FnOnce(&Session) -> T) -> AppResult<T> {
        lock(&self.inner.sessions)
            .get(&id)
            .map(f)
            .ok_or_else(|| AppError::NotFound(format!("terminal session {id}")))
    }
}
