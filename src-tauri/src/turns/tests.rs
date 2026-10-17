//! Turn extraction and live activity on fixture logs (shapes from docs/turn-signals.md,
//! values synthetic): multi-turn sessions, tool calls in between, interrupted turns, history
//! from before the panel, partial writes, and matching by CLI + folder.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::events::{ActivityKind, TurnEvent};
use super::signal::{claude_tool, codex_tool, relative, short, Action};
use super::watcher::TurnWatcher;
use crate::tokens::cli::{CliKind, CliRoots};
use crate::util::test_dir::TestDir;

const CLAUDE: &str = include_str!("fixtures/claude.jsonl");
const CODEX: &str = include_str!("fixtures/codex.jsonl");
const GEMINI: &str = include_str!("fixtures/gemini.jsonl");

struct Setup {
    dir: TestDir,
    work: PathBuf,
    turns: TurnWatcher,
    events: Arc<Mutex<Vec<TurnEvent>>>,
}

fn setup(label: &str) -> Setup {
    let dir = TestDir::new(label);
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).expect("work dir");
    let roots = CliRoots {
        claude: Some(dir.path().join("claude/projects")),
        codex: Some(dir.path().join("codex/sessions")),
        gemini: Some(dir.path().join("gemini/tmp")),
    };
    let turns = TurnWatcher::without_watch(roots);
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    turns.subscribe(Box::new(move |event| {
        sink.lock().expect("events").push(event)
    }));
    Setup {
        dir,
        work,
        turns,
        events,
    }
}

/// The fixture with `{cwd}` replaced by `cwd` as a JSON string body.
fn fixture(text: &str, cwd: &Path) -> String {
    let escaped = serde_json::to_string(&cwd.to_string_lossy()).expect("json");
    text.replace("{cwd}", escaped.trim_matches('"'))
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

fn append(path: &Path, text: &str) {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .expect("open");
    file.write_all(text.as_bytes()).expect("append");
}

impl Setup {
    fn take(&self) -> Vec<TurnEvent> {
        std::mem::take(&mut *self.events.lock().expect("events"))
    }
}

fn ended(events: &[TurnEvent]) -> Vec<(String, bool)> {
    events
        .iter()
        .filter_map(|e| match e {
            TurnEvent::TurnEnded {
                report,
                interrupted,
                ..
            } => Some((report.clone(), *interrupted)),
            _ => None,
        })
        .collect()
}

fn tools(events: &[TurnEvent]) -> Vec<(Action, Option<String>)> {
    events
        .iter()
        .filter_map(|e| match e {
            TurnEvent::Activity {
                kind: ActivityKind::Tool,
                action: Some(action),
                target,
                ..
            } => Some((*action, target.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn claude_turns_reports_and_tool_activity() {
    let s = setup("turns-claude");
    s.turns
        .panel_started("terminal-1", CliKind::Claude, &s.work);
    let file = s.dir.path().join("claude/projects/-work/s1.jsonl");
    write(&file, &fixture(CLAUDE, &s.work));
    s.turns.ingest(std::slice::from_ref(&file));
    let events = s.take();

    assert!(matches!(
        &events[0],
        TurnEvent::Attached { panel, exact: true } if panel == "terminal-1"
    ));
    assert_eq!(
        ended(&events),
        vec![
            (
                "Fixed the off-by-one in src/app.ts; all tests pass.".to_owned(),
                false
            ),
            (String::new(), true),
            ("Hi!".to_owned(), false),
        ],
        "history and sidechain text never become reports"
    );
    assert_eq!(
        tools(&events),
        vec![
            (Action::Run, Some("npm test".to_owned())),
            (Action::Edit, Some("src/app.ts".to_owned())),
            (Action::Run, Some("npm run lint".to_owned())),
        ]
    );
    assert!(events.iter().all(|e| !matches!(e, TurnEvent::Draft { .. })));
    let last = events.last().expect("events");
    assert!(matches!(
        last,
        TurnEvent::Activity {
            kind: ActivityKind::Idle,
            ..
        }
    ));
}

#[test]
fn an_end_waits_for_the_text_written_after_it() {
    let s = setup("turns-partial");
    s.turns
        .panel_started("terminal-1", CliKind::Claude, &s.work);
    let file = s.dir.path().join("claude/projects/-work/s1.jsonl");
    let all = fixture(CLAUDE, &s.work);
    let lines: Vec<&str> = all.lines().collect();
    // Up to the end_turn thinking line of msg_3 (its text block not written yet).
    let cut = lines
        .iter()
        .position(|l| l.contains("msg_3"))
        .expect("msg_3");
    write(&file, &format!("{}\n", lines[..=cut].join("\n")));
    s.turns.ingest(std::slice::from_ref(&file));
    assert!(ended(&s.take()).is_empty());
    // A partial line (no newline yet) is not read.
    append(&file, &lines[cut + 1][..20]);
    s.turns.ingest(std::slice::from_ref(&file));
    assert!(ended(&s.take()).is_empty());
    append(&file, &format!("{}\n", &lines[cut + 1][20..]));
    s.turns.ingest(std::slice::from_ref(&file));
    let reports = ended(&s.take());
    assert_eq!(reports.len(), 1);
    assert!(reports[0].0.starts_with("Fixed the off-by-one"));
}

#[test]
fn codex_turns_from_task_complete_and_abort() {
    let s = setup("turns-codex");
    s.turns.panel_started("terminal-2", CliKind::Codex, &s.work);
    let file = s
        .dir
        .path()
        .join("codex/sessions/2099/01/01/rollout-2099-01-01T00-00-00-t1.jsonl");
    write(&file, &fixture(CODEX, &s.work));
    s.turns.ingest(std::slice::from_ref(&file));
    let events = s.take();
    assert_eq!(
        ended(&events),
        vec![
            ("Fixed src/app.ts; tests pass.".to_owned(), false),
            (String::new(), true),
        ]
    );
    assert_eq!(
        tools(&events),
        vec![
            (Action::Run, Some("npm test".to_owned())),
            (Action::Edit, Some("src/app.ts".to_owned())),
            (Action::Run, Some("npm run lint".to_owned())),
        ]
    );
    let waits = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                TurnEvent::Activity {
                    kind: ActivityKind::Wait,
                    ..
                }
            )
        })
        .count();
    assert_eq!(waits, 1, "an approval request is a wait");
}

#[test]
fn gemini_sends_drafts_and_no_exact_end() {
    let s = setup("turns-gemini");
    s.turns
        .panel_started("terminal-3", CliKind::Gemini, &s.work);
    let project = s.dir.path().join("gemini/tmp/proj");
    write(&project.join(".project_root"), &s.work.to_string_lossy());
    let file = project.join("chats/session-2099-01-01T00-00-g1.jsonl");
    write(&file, GEMINI);
    s.turns.ingest(std::slice::from_ref(&file));
    let events = s.take();
    assert!(matches!(
        &events[0],
        TurnEvent::Attached { exact: false, .. }
    ));
    assert!(ended(&events).is_empty());
    assert_eq!(
        tools(&events),
        vec![(Action::Run, Some("npm test".to_owned()))]
    );
    let drafts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            TurnEvent::Draft { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(drafts, vec!["All tests pass now."]);
}

#[test]
fn logs_match_only_the_same_cli_in_the_same_folder() {
    let s = setup("turns-match");
    let other = s.dir.path().join("elsewhere");
    std::fs::create_dir_all(&other).expect("other");
    s.turns.panel_started("terminal-1", CliKind::Claude, &other);
    s.turns.panel_started("terminal-2", CliKind::Codex, &s.work);
    let file = s.dir.path().join("claude/projects/-work/s1.jsonl");
    write(&file, &fixture(CLAUDE, &s.work));
    s.turns.ingest(std::slice::from_ref(&file));
    assert!(s.take().is_empty(), "a Claude log in another folder");

    s.turns
        .panel_started("terminal-4", CliKind::Claude, &s.work);
    let second = s.dir.path().join("claude/projects/-work/s2.jsonl");
    write(&second, &fixture(CLAUDE, &s.work));
    s.turns.ingest(std::slice::from_ref(&second));
    let events = s.take();
    assert!(!events.is_empty());
    assert!(events.iter().all(|e| match e {
        TurnEvent::Attached { panel, .. }
        | TurnEvent::Activity { panel, .. }
        | TurnEvent::Draft { panel, .. }
        | TurnEvent::TurnEnded { panel, .. } => panel == "terminal-4",
    }));
}

#[test]
fn subagent_files_and_ended_panels_are_ignored() {
    let s = setup("turns-subagent");
    s.turns
        .panel_started("terminal-1", CliKind::Claude, &s.work);
    let side = s
        .dir
        .path()
        .join("claude/projects/-work/s1/subagents/agent-1.jsonl");
    write(&side, &fixture(CLAUDE, &s.work));
    s.turns.ingest(std::slice::from_ref(&side));
    assert!(s.take().is_empty());
}

#[test]
fn tool_inputs_become_short_targets() {
    let input = serde_json::json!({ "command": "npm test\n--watch=false" });
    assert_eq!(
        claude_tool("Bash", &input),
        (Action::Run, Some("npm test".into()))
    );
    let grep = serde_json::json!({ "pattern": "TODO" });
    assert_eq!(
        claude_tool("Grep", &grep),
        (Action::Search, Some("TODO".into()))
    );
    assert_eq!(
        claude_tool("mcp__x__y", &grep),
        (Action::Tool, Some("mcp__x__y".into()))
    );
    let argv = serde_json::json!({ "command": ["bash", "-lc", "cargo test"] });
    assert_eq!(
        codex_tool("shell", &argv),
        (Action::Run, Some("bash -lc cargo test".into()))
    );
    let long = "x".repeat(400);
    assert_eq!(short(&long).map(|s| s.chars().count()), Some(161));
    let root = if cfg!(windows) { "C:\\work" } else { "/work" };
    let inside = format!(
        "{root}{}src{}a.ts",
        std::path::MAIN_SEPARATOR,
        std::path::MAIN_SEPARATOR
    );
    assert_eq!(relative(inside, Some(root)), "src/a.ts");
    assert_eq!(relative("README.md".into(), Some(root)), "README.md");
}
