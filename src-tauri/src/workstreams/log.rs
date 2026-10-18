//! Managed state: appends handoffs to a workspace's JSONL file and reads one workstream's
//! entries back (newest `MAX_READ`, oldest first). Malformed lines are skipped.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::settings::HandoffMode;
use crate::util::{lock, workspace_key};

/// Longer texts are cut (a report is capped far below this before it is sent).
pub const MAX_TEXT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_READ: usize = 2_000;
const MAX_ID: usize = 80;
const MAX_NAME: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HandoffKind {
    /// Manager -> worker.
    Prompt,
    /// Worker -> manager.
    Report,
    /// The one-time manager instructions for a CLI manager.
    Setup,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffEntry {
    pub id: String,
    pub workstream: String,
    pub at: u64,
    /// Panel ids.
    pub from: String,
    pub to: String,
    pub from_name: String,
    pub to_name: String,
    pub kind: HandoffKind,
    pub mode: HandoffMode,
    /// Bytes of the full text.
    pub size: u64,
    pub tokens: Option<u64>,
    pub text: String,
    /// What was delivered was cut to the report cap; `text` is the whole.
    pub truncated: bool,
}

#[derive(Clone)]
pub struct WorkstreamLog {
    dir: PathBuf,
    /// Serializes appends to the same files.
    write: Arc<Mutex<()>>,
}

fn check(value: &str, max: usize, what: &str) -> AppResult<()> {
    if value.trim().is_empty() || value.len() > max {
        return Err(AppError::InvalidInput(format!("bad {what}")));
    }
    Ok(())
}

impl WorkstreamLog {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            write: Arc::new(Mutex::new(())),
        }
    }

    fn file(&self, workspace: Option<&str>) -> PathBuf {
        self.dir.join(format!("{}.jsonl", workspace_key(workspace)))
    }

    pub fn append(&self, workspace: Option<&str>, mut entry: HandoffEntry) -> AppResult<()> {
        check(&entry.id, MAX_ID, "id")?;
        check(&entry.workstream, MAX_ID, "workstream")?;
        check(&entry.from, MAX_ID, "panel")?;
        check(&entry.to, MAX_ID, "panel")?;
        entry.from_name = clip(&entry.from_name, MAX_NAME);
        entry.to_name = clip(&entry.to_name, MAX_NAME);
        entry.size = entry.text.len() as u64;
        entry.text = clip(&entry.text, MAX_TEXT_BYTES);
        let mut line = serde_json::to_vec(&entry)?;
        line.push(b'\n');
        let _guard = lock(&self.write);
        std::fs::create_dir_all(&self.dir)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.file(workspace))?;
        file.write_all(&line)?;
        Ok(())
    }

    /// One workstream's handoffs, oldest first (the newest `MAX_READ`).
    pub fn read(&self, workspace: Option<&str>, workstream: &str) -> AppResult<Vec<HandoffEntry>> {
        let file = match std::fs::File::open(self.file(workspace)) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err.into()),
        };
        let mut out: Vec<HandoffEntry> = Vec::new();
        for line in BufReader::new(file).lines() {
            let Ok(line) = line else { continue };
            match serde_json::from_str::<HandoffEntry>(&line) {
                Ok(entry) if entry.workstream == workstream => out.push(entry),
                Ok(_) => {}
                Err(err) => tracing::debug!("skipping a malformed workstream log line: {err}"),
            }
        }
        if out.len() > MAX_READ {
            out.drain(..out.len() - MAX_READ);
        }
        Ok(out)
    }
}

/// `text` cut to at most `max` bytes on a character boundary.
fn clip(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_owned();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_dir::TestDir;

    fn entry(id: &str, workstream: &str, text: &str) -> HandoffEntry {
        HandoffEntry {
            id: id.into(),
            workstream: workstream.into(),
            at: 1,
            from: "chat-1".into(),
            to: "terminal-2".into(),
            from_name: "Owl".into(),
            to_name: "Fox".into(),
            kind: HandoffKind::Prompt,
            mode: HandoffMode::Auto,
            size: 0,
            tokens: None,
            text: text.into(),
            truncated: false,
        }
    }

    #[test]
    fn keeps_full_texts_per_workstream_and_workspace() {
        let dir = TestDir::new("workstream-log");
        let log = WorkstreamLog::new(dir.path().to_path_buf());
        let report = "line\n".repeat(20_000);
        log.append(Some("/w"), entry("h1", "ws-a", &report))
            .expect("append");
        log.append(Some("/w"), entry("h2", "ws-b", "other stream"))
            .expect("append");
        log.append(Some("/elsewhere"), entry("h3", "ws-a", "other workspace"))
            .expect("append");
        // A torn line from a crash is skipped.
        let file = dir
            .path()
            .join(format!("{}.jsonl", workspace_key(Some("/w"))));
        let mut raw = std::fs::read(&file).expect("read");
        raw.extend_from_slice(b"{\"id\":\"broken\n");
        std::fs::write(&file, raw).expect("write");

        let read = log.read(Some("/w"), "ws-a").expect("read");
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].text, report, "the log has the whole report");
        assert_eq!(read[0].size, report.len() as u64);
        assert!(log.read(Some("/w"), "nope").expect("read").is_empty());
        assert!(log.read(None, "ws-a").expect("read").is_empty());
    }

    #[test]
    fn rejects_bad_ids() {
        let dir = TestDir::new("workstream-log-bad");
        let log = WorkstreamLog::new(dir.path().to_path_buf());
        assert!(log.append(None, entry(" ", "ws", "x")).is_err());
        assert!(log.append(None, entry("h", &"w".repeat(200), "x")).is_err());
        assert_eq!(clip("añb", 2), "a");
    }
}
