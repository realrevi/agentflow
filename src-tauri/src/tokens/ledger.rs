//! The usage ledger on disk: append-only JSONL, one file per UTC month
//! (`<app data>/usage/2026-10.jsonl`). Lines are never rewritten; a malformed line is skipped
//! with a debug log, so a crash mid-write can never take the history down with it.

use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use super::record::UsageRecord;
use super::time::month_key;
use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct LedgerFiles {
    dir: PathBuf,
}

impl LedgerFiles {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Appends records to their month files (one write per file).
    pub fn append(&self, records: &[UsageRecord]) -> AppResult<()> {
        if records.is_empty() {
            return Ok(());
        }
        fs::create_dir_all(&self.dir)?;
        let mut by_month: Vec<(String, Vec<u8>)> = Vec::new();
        for record in records {
            let month = month_key(record.at);
            let mut line = serde_json::to_vec(record)?;
            line.push(b'\n');
            match by_month.iter_mut().find(|(m, _)| *m == month) {
                Some((_, buffer)) => buffer.extend_from_slice(&line),
                None => by_month.push((month, line)),
            }
        }
        for (month, buffer) in by_month {
            let path = self.dir.join(format!("{month}.jsonl"));
            let mut file = OpenOptions::new()
                .create(true)
                .read(true)
                .append(true)
                .open(&path)?;
            // A torn last line (crash mid-write) must not swallow the next record.
            if !ends_with_newline(&mut file)? {
                file.write_all(b"\n")?;
            }
            file.write_all(&buffer)?;
            file.flush()?;
        }
        Ok(())
    }

    /// Every record of the month files from `since` (e.g. "2025-10") on, oldest file first.
    pub fn load(&self, since: &str) -> Vec<UsageRecord> {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().is_some_and(|ext| ext == "jsonl")
                    && path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .is_some_and(|stem| stem.len() == 7 && stem >= since)
            })
            .collect();
        files.sort();
        let mut out = Vec::new();
        for path in files {
            match fs::read_to_string(&path) {
                Ok(text) => out.extend(parse_lines(&text, &path)),
                Err(err) => tracing::warn!("cannot read {}: {err}", path.display()),
            }
        }
        out
    }

    pub fn has_marker(&self, name: &str) -> bool {
        self.dir.join(name).exists()
    }

    pub fn write_marker(&self, name: &str) -> AppResult<()> {
        fs::create_dir_all(&self.dir)?;
        fs::write(self.dir.join(name), b"done\n")?;
        Ok(())
    }
}

fn parse_lines(text: &str, path: &Path) -> Vec<UsageRecord> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| match serde_json::from_str::<UsageRecord>(line) {
            Ok(record) => Some(record),
            Err(err) => {
                tracing::debug!(
                    "skipping a malformed ledger line in {}: {err}",
                    path.display()
                );
                None
            }
        })
        .collect()
}

fn ends_with_newline(file: &mut fs::File) -> std::io::Result<bool> {
    let len = file.metadata()?.len();
    if len == 0 {
        return Ok(true);
    }
    file.seek(SeekFrom::Start(len - 1))?;
    let mut last = [0u8; 1];
    file.read_exact(&mut last)?;
    Ok(last[0] == b'\n')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::record::{sample, Source};
    use crate::tokens::time::parse_iso;
    use crate::util::test_dir::TestDir;

    #[test]
    fn appends_per_month_and_skips_bad_lines() {
        let dir = TestDir::new("ledger");
        let ledger = LedgerFiles::new(dir.path().join("usage"));
        let sept = parse_iso("2026-09-30T23:00:00Z").expect("date");
        let oct = parse_iso("2026-10-02T10:00:00Z").expect("date");
        ledger
            .append(&[
                sample(sept, Source::Api, "m1", 10, 5),
                sample(oct, Source::Codex, "m2", 20, 7),
            ])
            .expect("append");
        // A torn line from a crash, then more records.
        let oct_file = dir.path().join("usage/2026-10.jsonl");
        let mut file = OpenOptions::new()
            .append(true)
            .open(&oct_file)
            .expect("open");
        file.write_all(b"{\"at\":12,\"sour").expect("torn");
        ledger
            .append(&[sample(oct + 1, Source::Api, "m3", 1, 1)])
            .expect("append");

        let all = ledger.load("2026-01");
        let models: Vec<&str> = all.iter().map(|r| r.model.as_str()).collect();
        assert_eq!(models, ["m1", "m2", "m3"]);
        assert_eq!(ledger.load("2026-10").len(), 2);
        assert!(dir.path().join("usage/2026-09.jsonl").exists());
    }

    #[test]
    fn markers() {
        let dir = TestDir::new("ledger-marker");
        let ledger = LedgerFiles::new(dir.path().join("usage"));
        assert!(!ledger.has_marker(".backfill"));
        ledger.write_marker(".backfill").expect("marker");
        assert!(ledger.has_marker(".backfill"));
        assert!(ledger.load("2000-01").is_empty());
    }
}
