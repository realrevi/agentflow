//! Pseudo-terminal sessions for terminal panels (portable-pty; ConPTY on Windows).
//!
//! Each session has three threads: a reader (pty -> queue), a waiter (reaps the child and
//! reports its exit) and a pump that coalesces output (~10 ms / 64 KB) into an
//! [`OutputSink`], which the command layer backs with a `tauri::ipc::Channel`. The reader
//! pauses while too much output is unacknowledged by the frontend (see `flow`).

mod cli;
mod coalesce;
mod env_path;
mod flow;
mod kill;
mod launch;
mod manager;
mod model;
mod session;
mod shell;
#[cfg(all(test, unix))]
mod tests;

pub use cli::{CliDetector, CliStatus};
pub use launch::{resolve_launch, Launch};
pub use manager::PtyManager;
pub use model::{ExitInfo, OutputSink, PtyKind, PtyOutput, SessionId, SessionInfo, TermSize};
