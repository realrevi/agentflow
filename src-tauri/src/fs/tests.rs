//! End-to-end tests of the workspace API (what the commands call).

use std::fs;
use std::path::Path;

use super::model::EntryKind;
use super::workspace::{FsState, Workspace};
use crate::error::AppError;
use crate::util::test_dir::TestDir;

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, text).expect("write");
}

/// A small project with ignored, hidden and git files.
fn project() -> TestDir {
    let dir = TestDir::new("fs-workspace");
    let root = dir.path();
    write(root, ".gitignore", "target/\n*.log\n");
    write(root, ".git/config", "[core]");
    write(root, ".env", "SECRET=1");
    write(root, "target/debug/app", "bin");
    write(root, "build.log", "noise");
    write(root, "src/main.rs", "fn main() {}");
    write(root, "src/lib.rs", "");
    write(root, "src/ui/layoutStore.ts", "");
    write(root, "README.md", "# hi");
    dir
}

fn open(dir: &TestDir, show_hidden: bool) -> Workspace {
    Workspace::open(dir.path(), show_hidden, |_| {}).expect("open")
}

fn names(workspace: &Workspace, dir: &str) -> Vec<String> {
    let entries = workspace.list(dir).expect("list");
    entries.into_iter().map(|entry| entry.name).collect()
}

#[test]
fn listing_respects_gitignore_and_hides_git() {
    let dir = project();
    let workspace = open(&dir, false);
    assert_eq!(names(&workspace, ""), ["src", "README.md"]);
    assert_eq!(names(&workspace, "src"), ["ui", "lib.rs", "main.rs"]);
    let nested = workspace.list("src/ui").expect("list");
    assert_eq!(nested[0].path, "src/ui/layoutStore.ts");

    let with_hidden = open(&dir, true);
    assert_eq!(
        names(&with_hidden, ""),
        ["src", ".env", ".gitignore", "README.md"],
        ".git stays hidden and ignored files stay out"
    );
}

#[test]
fn quick_open_respects_gitignore_and_ranks_matches() {
    let dir = project();
    let workspace = open(&dir, false);
    let all: Vec<String> = workspace
        .quick_open("")
        .into_iter()
        .map(|m| m.path)
        .collect();
    assert_eq!(
        all,
        [
            "README.md",
            "src/lib.rs",
            "src/main.rs",
            "src/ui/layoutStore.ts"
        ]
    );

    let hits = workspace.quick_open("lstore");
    assert_eq!(hits[0].path, "src/ui/layoutStore.ts");
    assert!(workspace.quick_open("app").is_empty(), "target/ is ignored");
    assert!(workspace.quick_open("build").is_empty(), "*.log is ignored");

    // The cache is dropped when files are created through the workspace.
    workspace
        .create("src/fresh.rs", EntryKind::File)
        .expect("create");
    assert_eq!(workspace.quick_open("fresh")[0].path, "src/fresh.rs");
}

#[test]
fn paths_outside_the_workspace_are_rejected() {
    let dir = project();
    let workspace = open(&dir, false);
    for bad in [
        "../outside-secret.txt",
        "src/../../outside-secret.txt",
        "/etc/passwd",
    ] {
        assert!(matches!(
            workspace.read(bad),
            Err(AppError::InvalidInput(_))
        ));
        assert!(workspace.write(bad, "x", None).is_err());
        assert!(workspace.list(bad).is_err());
        assert!(workspace.delete(bad).is_err());
        assert!(workspace.create(bad, EntryKind::File).is_err());
        assert!(workspace.rename("README.md", bad).is_err());
    }
    assert!(
        workspace.delete("").is_err(),
        "the root itself is protected"
    );
}

#[cfg(unix)]
#[test]
fn symlinks_escaping_the_root_are_hidden_and_rejected() {
    use std::os::unix::fs::symlink;
    let dir = project();
    let outside = TestDir::new("fs-outside");
    write(outside.path(), "secret.txt", "nope");
    symlink(outside.path(), dir.path().join("escape")).expect("link");
    symlink(dir.path().join("src"), dir.path().join("src-link")).expect("link");
    let workspace = open(&dir, false);

    let listed = names(&workspace, "");
    assert!(!listed.contains(&"escape".to_string()), "{listed:?}");
    assert!(listed.contains(&"src-link".to_string()), "{listed:?}");
    assert!(matches!(
        workspace.read("escape/secret.txt"),
        Err(AppError::InvalidInput(_))
    ));
    assert!(workspace.write("escape/new.txt", "x", None).is_err());
    assert!(!outside.path().join("new.txt").exists());
    assert!(workspace.read("src-link/main.rs").is_ok());
}

#[test]
fn write_create_rename_round_trip() {
    let dir = project();
    let workspace = open(&dir, false);
    let read = workspace.read("README.md").expect("read");
    assert_eq!(read.content.as_deref(), Some("# hi"));

    let saved = workspace
        .write("README.md", "# hello", Some(read.mtime))
        .expect("save");
    assert!(matches!(
        workspace.write(
            "README.md",
            "# stale",
            Some(saved.mtime.saturating_sub(5_000))
        ),
        Err(AppError::Conflict(_))
    ));

    let folder = workspace.create("docs", EntryKind::Dir).expect("mkdir");
    assert!(folder.is_dir);
    let moved = workspace
        .rename("README.md", "docs/README.md")
        .expect("move");
    assert_eq!(moved.path, "docs/README.md");
    assert_eq!(names(&workspace, "docs"), ["README.md"]);
    assert!(matches!(
        workspace.read("README.md"),
        Err(AppError::NotFound(_))
    ));
}

#[test]
fn state_installs_only_the_latest_workspace() {
    let dir = project();
    let state = FsState::new();
    assert!(matches!(state.current(), Err(AppError::NotFound(_))));
    let first = state.ticket();
    let second = state.ticket();
    assert!(state.install(second, Some(open(&dir, false))));
    assert!(!state.install(first, None), "an older open never wins");
    assert!(state.current().is_ok());
    state.close();
    assert!(state.current().is_err());
}
