use std::io::Write;

use super::*;

#[test]
fn offsets_survive_a_restart_and_partial_lines_wait() {
    let dir = TestDir::new("cli-offsets");
    let path = dir.path().join("projects/-work-demo/s1.jsonl");
    let cut = CLAUDE.find("{\"type\":\"summary\"").expect("cut");
    let (head, tail) = CLAUDE.split_at(cut);
    // The last line is still being written: no newline yet.
    let (partial, rest) = tail.split_at(20);
    write(&path, &format!("{head}{partial}"));
    let offsets = dir.path().join("cli-offsets.json");
    let mut store = OffsetStore::load(offsets.clone());
    let mut seen = HashSet::new();
    assert_eq!(
        read_new(CliKind::Claude, &path, &mut store, &mut seen).len(),
        1
    );
    store.save();

    // Restart: a fresh store from disk and a fresh dedup set (the worst case).
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("open");
    file.write_all(rest.as_bytes()).expect("append");
    let mut store = OffsetStore::load(offsets);
    let mut seen = HashSet::new();
    let usage = read_new(CliKind::Claude, &path, &mut store, &mut seen);
    let ids: Vec<&str> = usage.iter().map(|u| u.id.as_str()).collect();
    assert_eq!(
        ids,
        ["msg_2"],
        "nothing before the stored offset is read again"
    );
}

#[test]
fn a_truncated_file_is_read_again_from_the_start() {
    let dir = TestDir::new("cli-truncate");
    let path = dir.path().join("projects/-w/s.jsonl");
    write(&path, CLAUDE);
    let mut store = OffsetStore::default();
    let mut seen = HashSet::new();
    assert_eq!(
        read_new(CliKind::Claude, &path, &mut store, &mut seen).len(),
        2
    );
    let first_two = CLAUDE.lines().take(2).collect::<Vec<_>>().join("\n") + "\n";
    write(&path, &first_two);
    // Same ids: dedup keeps the total unchanged.
    assert!(read_new(CliKind::Claude, &path, &mut store, &mut seen).is_empty());
    assert!(store.get(&path).offset > 0);
}
