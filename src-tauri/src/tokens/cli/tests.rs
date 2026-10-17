//! The CLI readers on fixture files (shapes from each CLI's source, values synthetic):
//! parse, dedup, malformed lines, and offsets that survive a restart.

use std::collections::HashSet;
use std::path::Path;

use super::*;
use crate::util::test_dir::TestDir;

mod formats;
mod tail;

const CLAUDE: &str = include_str!("fixtures/claude.jsonl");
const CODEX: &str = include_str!("fixtures/codex.jsonl");
const CODEX_OLD: &str = include_str!("fixtures/codex_old.jsonl");
const GEMINI: &str = include_str!("fixtures/gemini.jsonl");

/// Reads like the tracker does: offsets from `store`, ids deduplicated in `seen`.
fn read_new(
    kind: CliKind,
    path: &Path,
    store: &mut OffsetStore,
    seen: &mut HashSet<String>,
) -> Vec<LineUsage> {
    let result = read_file(kind, path, store.get(path)).expect("read");
    store.set(path, result.state);
    result
        .usage
        .into_iter()
        .filter(|u| seen.insert(u.id.clone()))
        .collect()
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

#[test]
fn roots_and_session_files() {
    let roots = CliRoots {
        claude: Some("/h/.claude/projects".into()),
        codex: Some("/h/.codex/sessions".into()),
        gemini: Some("/h/.gemini/tmp".into()),
    };
    let kind = |p: &str| roots.kind_of(Path::new(p));
    assert_eq!(
        kind("/h/.claude/projects/-w/s.jsonl"),
        Some(CliKind::Claude)
    );
    assert_eq!(
        kind("/h/.codex/sessions/2026/10/05/rollout-x.jsonl"),
        Some(CliKind::Codex)
    );
    assert_eq!(
        kind("/h/.codex/sessions/2026/10/05/rollout-x.jsonl.zst"),
        None
    );
    assert_eq!(
        kind("/h/.gemini/tmp/p/chats/session-1.jsonl"),
        Some(CliKind::Gemini)
    );
    assert_eq!(kind("/h/.gemini/tmp/p/logs.json"), None);
    assert_eq!(kind("/elsewhere/s.jsonl"), None);
}
