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

/// Calls `line` for every complete, non-empty UTF-8 line from byte `offset` on (a line
/// still being written is left for the next read) and returns the offset after the last
/// one. Oversized lines are skipped.
pub fn tail_lines(path: &Path, offset: u64, mut line: impl FnMut(&str)) -> std::io::Result<u64> {
    let mut reader = BufReader::new(File::open(path)?);
    reader.seek(SeekFrom::Start(offset))?;
    let mut offset = offset;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let read = reader.read_until(b'\n', &mut buf)?;
        if read == 0 || buf.last() != Some(&b'\n') {
            return Ok(offset); // end of file, or a partial line still being written
        }
        offset += read as u64;
        if buf.len() > MAX_LINE {
            tracing::debug!("skipping an oversized line in {}", path.display());
            continue;
        }
        let Ok(text) = std::str::from_utf8(&buf) else {
            tracing::debug!("skipping a non-UTF-8 line in {}", path.display());
            continue;
        };
        let text = text.trim();
        if !text.is_empty() {
            line(text);
        }
    }
}

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
    let mut usage = Vec::new();
    let mut has_records = state.has_records;
    let (mut cwd, mut model) = (state.cwd.take(), state.model.take());
    state.offset = tail_lines(path, state.offset, |text| {
        if kind == CliKind::Codex && text.contains("\"token_usage_record\"") {
            has_records = true;
        }
        let parsed = match kind {
            CliKind::Claude => claude_line(text),
            CliKind::Codex => codex_line(text, has_records),
            CliKind::Gemini => gemini_line(text),
        };
        match parsed {
            Ok(Parsed::Usage(mut found)) => {
                found.model = found.model.or_else(|| model.clone());
                found.cwd = found.cwd.or_else(|| cwd.clone());
                usage.push(found);
            }
            Ok(Parsed::Context { cwd: c, model: m }) => {
                cwd = c.or(cwd.take());
                model = m.or(model.take());
            }
            Ok(Parsed::Nothing) => {}
            Err(err) => tracing::debug!("skipping a malformed line in {}: {err}", path.display()),
        }
    })?;
    state.has_records = has_records;
    (state.cwd, state.model) = (cwd, model);
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
