//! One running pty: the handles kept in the manager, and the threads that serve it.

use std::io::{Read, Write};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use portable_pty::{Child, ChildKiller, MasterPty};

use super::coalesce::PumpMsg;
use super::model::{ExitInfo, SessionInfo};
use crate::util::Signal;

const READ_BUFFER: usize = 16 * 1024;

/// Handles for a live session. Dropping it closes the pty master, which hangs up the
/// terminal and ends the reader thread.
pub struct Session {
    pub info: SessionInfo,
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub master: Arc<Mutex<Box<dyn MasterPty + Send>>>,
    pub killer: Box<dyn ChildKiller + Send + Sync>,
    /// Set by the waiter thread once the child has been reaped.
    pub reaped: Arc<Signal>,
}

/// Reader thread body: forwards raw bytes until EOF or an error (EIO once the child
/// and every process holding the pty slave are gone).
pub fn read_loop(mut reader: Box<dyn Read + Send>, tx: Sender<PumpMsg>) {
    let mut buffer = vec![0u8; READ_BUFFER];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if tx.send(PumpMsg::Data(buffer[..n].to_vec())).is_err() {
                    break;
                }
            }
        }
    }
}

/// Waiter thread body: blocks until the child exits, reaps it, then reports the exit.
pub fn wait_loop(mut child: Box<dyn Child + Send + Sync>, reaped: &Signal, tx: &Sender<PumpMsg>) {
    let info = match child.wait() {
        Ok(status) => ExitInfo {
            code: Some(status.exit_code()),
            signal: status.signal().map(str::to_owned),
        },
        Err(err) => {
            tracing::warn!("waiting for terminal process failed: {err}");
            ExitInfo {
                code: None,
                signal: None,
            }
        }
    };
    reaped.set();
    let _ = tx.send(PumpMsg::Exited(info));
}
