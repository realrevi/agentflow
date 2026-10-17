//! Reads what is new in one session file: complete lines from the stored offset on (a line
//! still being written is left for the next read). Legacy Gemini `.json` files are whole
//! documents, re-read when they change; ids keep that from double counting.

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::time::UNIX_EPOCH;

use super::offsets::FileState;
use super::parse::{claude_line, codex_line, gemini_line, gemini_message, LineUsage, Parsed};
use super::CliKind;

/// Lines longer than this are skipped (a huge tool result cannot hold usage we need).
const MAX_LINE: usize = 16 * 1024 * 1024;

pub struct ReadResult {
    pub usage: Vec<LineUsage>,
    pub state: FileState,
}

/// Reads new usage from `path`, given its previous state.
pub fn read_file(kind: CliKind, path: &Path, mut state: FileState) -> std::io::Result<ReadResult> {
    let meta = std::fs::metadata(path)?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as u64);
    let len = meta.len();
    let legacy_json = path.extension().is_some_and(|ext| ext == "json");
    if legacy_json {
        if state.mtime == mtime && state.offset == len {
            return Ok(ReadResult {
                usage: Vec::new(),
                state,
            });
        }
        let usage = read_legacy_gemini(path);
        state.offset = len;
        state.mtime = mtime;
        return Ok(ReadResult { usage, state });
    }
    if len < state.offset {
        // Truncated or replaced: start over (ids prevent double counts).
        state = FileState::default();
    }
    let mut reader = BufReader::new(File::open(path)?);
    reader.seek(SeekFrom::Start(state.offset))?;
    let mut usage = Vec::new();
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = reader.read_until(b'\n', &mut line)?;
        if read == 0 || line.last() != Some(&b'\n') {
            break; // end of file, or a partial line still being written
        }
        state.offset += read as u64;
        if line.len() > MAX_LINE {
            tracing::debug!("skipping an oversized line in {}", path.display());
            continue;
        }
        let Ok(text) = std::str::from_utf8(&line) else {
            tracing::debug!("skipping a non-UTF-8 line in {}", path.display());
            continue;
        };
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        if kind == CliKind::Codex && text.contains("\"token_usage_record\"") {
            state.has_records = true;
        }
        let parsed = match kind {
            CliKind::Claude => claude_line(text),
            CliKind::Codex => codex_line(text, state.has_records),
            CliKind::Gemini => gemini_line(text),
        };
        match parsed {
            Ok(Parsed::Usage(mut found)) => {
                found.model = found.model.or_else(|| state.model.clone());
                found.cwd = found.cwd.or_else(|| state.cwd.clone());
                usage.push(found);
            }
            Ok(Parsed::Context { cwd, model }) => {
                state.cwd = cwd.or(state.cwd.take());
                state.model = model.or(state.model.take());
            }
            Ok(Parsed::Nothing) => {}
            Err(err) => tracing::debug!("skipping a malformed line in {}: {err}", path.display()),
        }
    }
    state.mtime = mtime;
    Ok(ReadResult { usage, state })
}

fn read_legacy_gemini(path: &Path) -> Vec<LineUsage> {
    let parsed = std::fs::read(path)
        .map_err(|err| err.to_string())
        .and_then(|bytes| {
            serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|err| err.to_string())
        });
    match parsed {
        Ok(doc) => doc["messages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(gemini_message)
            .collect(),
        Err(err) => {
            tracing::debug!("skipping an unreadable session {}: {err}", path.display());
            Vec::new()
        }
    }
}
