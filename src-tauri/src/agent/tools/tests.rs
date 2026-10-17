//! The tools against a temporary workspace: results, sandbox escapes, edit rules,
//! commands (output, exit code, timeout and cancel kill), checkpoints and truncation.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::*;
use crate::agent::checkpoint::CheckpointStore;
use crate::providers::cancel_pair;
use crate::util::test_dir::TestDir;

struct Fixture {
    dir: TestDir,
    env: ToolEnv,
    checkpoints: CheckpointStore,
}

fn fixture() -> Fixture {
    let dir = TestDir::new("agent-tools");
    let work = dir.path().join("work");
    std::fs::create_dir_all(work.join("src/ui")).expect("mkdir");
    std::fs::write(work.join(".gitignore"), "target/\n").expect("write");
    std::fs::write(work.join("README.md"), "# demo\nline two\n").expect("write");
    std::fs::write(
        work.join("src/main.rs"),
        "fn main() {\n    println!(\"hi\");\n}\n",
    )
    .expect("write");
    std::fs::write(
        work.join("src/ui/app.ts"),
        "export const a = 1;\nexport const b = 1;\n",
    )
    .expect("write");
    std::fs::create_dir_all(work.join("target")).expect("mkdir");
    std::fs::write(work.join("target/out.rs"), "fn ignored() {}").expect("write");
    std::fs::write(dir.path().join("secret.txt"), "outside").expect("write");
    let checkpoints = CheckpointStore::new(dir.path().join("checkpoints"));
    let root = WorkspaceRoot::open(&work).expect("root");
    let checkpoint = checkpoints.turn(Some("w"), "c1", "m1", root.path());
    Fixture {
        env: ToolEnv {
            root,
            cap: 20_000,
            timeout: Duration::from_secs(30),
            checkpoint,
        },
        checkpoints,
        dir,
    }
}

fn call(name: &str, input: Value) -> ToolCall {
    ToolCall {
        id: "call_1".into(),
        name: name.into(),
        input,
    }
}

fn run_tool(fixture: &Fixture, name: &str, input: Value) -> ToolOutput {
    let (_handle, cancel) = cancel_pair();
    tauri::async_runtime::block_on(run(&fixture.env, &call(name, input), &cancel))
}

#[test]
fn read_tools_list_read_glob_and_search() {
    let f = fixture();
    let listed = run_tool(&f, "list_dir", json!({ "path": ".", "depth": 2 }));
    assert!(!listed.is_error, "{}", listed.content);
    assert!(listed.content.contains("src/ui/") && listed.content.contains("README.md"));
    assert!(
        !listed.content.contains("target"),
        "gitignored: {}",
        listed.content
    );

    let read = run_tool(
        &f,
        "read_file",
        json!({ "path": "src/main.rs", "offset": 2, "limit": 1 }),
    );
    assert_eq!(
        read.content.lines().next(),
        Some("     2\t    println!(\"hi\");")
    );
    assert!(read.content.contains("1 more lines"));
    assert_eq!(read.meta.lines, Some([2, 2, 3]));

    let glob = run_tool(&f, "glob", json!({ "pattern": "**/*.rs" }));
    assert_eq!(glob.content, "src/main.rs", "target/ is ignored");
    let search = run_tool(
        &f,
        "search",
        json!({ "regex": "export const \\w", "glob": "**/*.ts" }),
    );
    assert_eq!((search.meta.count, search.meta.files), (Some(2), Some(1)));
    assert!(search
        .content
        .starts_with("src/ui/app.ts:1: export const a = 1;"));
}

#[test]
fn every_tool_rejects_paths_outside_the_workspace() {
    let f = fixture();
    let outside = f
        .dir
        .path()
        .join("secret.txt")
        .to_string_lossy()
        .into_owned();
    let escapes = ["../secret.txt", "src/../../secret.txt", outside.as_str()];
    for path in escapes {
        let attempts = [
            ("list_dir", json!({ "path": path })),
            ("read_file", json!({ "path": path })),
            ("search", json!({ "regex": "x", "path": path })),
            ("write_file", json!({ "path": path, "content": "pwned" })),
            (
                "edit_file",
                json!({ "path": path, "old_string": "outside", "new_string": "pwned" }),
            ),
        ];
        for (tool, input) in attempts {
            let output = run_tool(&f, tool, input);
            assert!(
                output.is_error,
                "{tool} {path} should fail: {}",
                output.content
            );
        }
    }
    assert_eq!(
        std::fs::read_to_string(f.dir.path().join("secret.txt")).expect("read"),
        "outside"
    );
}

#[test]
fn edit_file_needs_one_exact_match_unless_replace_all() {
    let f = fixture();
    let path = f.env.root.path().join("src/ui/app.ts");
    let edit = |old: &str, new: &str, all: bool| {
        run_tool(
            &f,
            "edit_file",
            json!({ "path": "src/ui/app.ts", "old_string": old, "new_string": new, "replace_all": all }),
        )
    };
    let missing = edit("const c", "const d", false);
    assert!(missing.is_error && missing.content.contains("not found"));
    let twice = edit("= 1;", "= 2;", false);
    assert!(twice.is_error && twice.content.contains("appears 2 times"));
    let exact = edit("const a = 1;", "const a = 10;", false);
    assert!(!exact.is_error, "{}", exact.content);
    assert_eq!(exact.meta.count, Some(1));
    let all = edit("export", "export default", true);
    assert!(all.content.contains("2 replacements"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read"),
        "export default const a = 10;\nexport default const b = 1;\n"
    );
}

#[test]
fn writes_are_checkpointed_and_can_be_undone() {
    let f = fixture();
    let created = run_tool(
        &f,
        "write_file",
        json!({ "path": "notes/todo.md", "content": "- one\n" }),
    );
    assert_eq!(created.meta.created, Some(true));
    let edited = run_tool(
        &f,
        "edit_file",
        json!({ "path": "README.md", "old_string": "line two", "new_string": "line 2" }),
    );
    assert!(!edited.is_error);
    let report = f
        .checkpoints
        .restore(Some("w"), "c1", "m1", |p| Ok(std::fs::remove_file(p)?))
        .expect("restore");
    assert_eq!(report.restored, ["README.md"]);
    assert_eq!(report.removed, ["notes/todo.md"]);
    let root = f.env.root.path();
    assert_eq!(
        std::fs::read_to_string(root.join("README.md")).expect("read"),
        "# demo\nline two\n"
    );
    assert!(!root.join("notes/todo.md").exists());
}

#[test]
fn previews_show_before_and_after() {
    let f = fixture();
    let edit = call(
        "edit_file",
        json!({ "path": "README.md", "old_string": "line two", "new_string": "line 2" }),
    );
    let shown = preview(&f.env, &edit).expect("preview");
    assert_eq!(shown.before.as_deref(), Some("# demo\nline two\n"));
    assert_eq!(shown.after, "# demo\nline 2\n");
    let new = call("write_file", json!({ "path": "new.txt", "content": "x" }));
    assert_eq!(preview(&f.env, &new).expect("preview").before, None);
}

/// A command that prints, writes to stderr and exits with 3, in the shell's own syntax.
fn exit_three() -> &'static str {
    if cfg!(windows) {
        "Write-Output 'out-line'; [Console]::Error.WriteLine('err-line'); exit 3"
    } else {
        "echo out-line; echo err-line >&2; exit 3"
    }
}

fn long_sleep() -> &'static str {
    if cfg!(windows) {
        "Start-Sleep -Seconds 30"
    } else {
        "sleep 30"
    }
}

#[test]
fn run_command_captures_output_and_exit_code() {
    let f = fixture();
    let output = run_tool(&f, "run_command", json!({ "command": exit_three() }));
    assert!(output.is_error, "exit 3 is an error result");
    assert_eq!(output.meta.exit_code, Some(3));
    assert!(output.content.contains("out-line"), "{}", output.content);
    assert!(
        output.content.contains("--- stderr ---\nerr-line"),
        "{}",
        output.content
    );
}

#[test]
fn run_command_is_killed_on_timeout_and_on_cancel() {
    let f = fixture();
    let started = Instant::now();
    let output = run_tool(
        &f,
        "run_command",
        json!({ "command": long_sleep(), "timeout": 1 }),
    );
    assert!(
        output.content.contains("timed out after 1 s"),
        "{}",
        output.content
    );
    assert!(started.elapsed() < Duration::from_secs(15));

    let (handle, cancel) = cancel_pair();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(700));
        handle.cancel();
    });
    let started = Instant::now();
    let output = tauri::async_runtime::block_on(run(
        &f.env,
        &call("run_command", json!({ "command": long_sleep() })),
        &cancel,
    ));
    canceller.join().expect("join");
    assert!(output.content.contains("stopped"), "{}", output.content);
    assert!(started.elapsed() < Duration::from_secs(15));
}

#[test]
fn bad_input_and_truncation() {
    let f = fixture();
    let raw = run_tool(&f, "read_file", json!({ "_raw": "{\"path\": " }));
    assert!(raw.is_error && raw.content.contains("not valid JSON"));
    assert!(run_tool(&f, "read_file", json!({}))
        .content
        .contains("Missing \"path\""));
    assert!(run_tool(&f, "nope", json!({})).is_error);
    assert_eq!(
        truncate("héllo world", 3),
        "h\u{e9}\n[truncated, 9 more bytes]"
    );
    assert_eq!(truncate("short", 100), "short");
}

#[test]
fn read_only_offers_read_tools_only() {
    let names = |mode| specs(mode).into_iter().map(|t| t.name).collect::<Vec<_>>();
    assert_eq!(
        names(PermissionMode::ReadOnly),
        ["list_dir", "read_file", "glob", "search"]
    );
    assert_eq!(names(PermissionMode::Ask).len(), 7);
}
