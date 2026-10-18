//! Workstream log (phase 9): every handoff between a manager and its workers, per
//! workspace in the app data dir (`workstreams/<workspace key>.jsonl`, append-only). The
//! workstreams themselves persist with the layout (frontend); handoff text is data: it is
//! stored and shown, never run.

mod log;

pub use log::{HandoffEntry, WorkstreamLog};
