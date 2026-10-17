//! Self-cleaning temporary directory for tests (avoids a `tempfile` dependency).
//!
//! Known issue (Windows, open): about once in 35-150 runs of the whole suite in parallel, one
//! test that writes files under `%TEMP%` stalls for minutes (seen: fs
//! `write_create_rename_round_trip` most often, prompts `corrupt_files_start_empty`); it never
//! happens when that test runs alone. Ruled out: the trash/COM test (hangs with it skipped),
//! the notify watcher (the prompts test has none), ignore's parallel walker (every walk
//! finished), std's `remove_dir_all` retries (bounded), the mock server port race (fixed in
//! `providers/tests/mock.rs`). What the stalled tests share is plain file writes through
//! `util::write_atomic` (create, `sync_all` = FlushFileBuffers, rename) while ~200 tests write
//! at once; antivirus filter drivers are the main suspect. Not marked serial: the stalled test
//! differs between runs, so serializing one would hide nothing. Rerun if a run hangs.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct TestDir(PathBuf);

impl TestDir {
    pub fn new(label: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("agentflow-test-{label}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create test dir");
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
