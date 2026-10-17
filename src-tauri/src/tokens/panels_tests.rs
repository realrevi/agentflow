use super::*;

fn session(panel: &str, kind: CliKind, cwd: &str, started: u64) -> PanelSession {
    PanelSession {
        panel: panel.into(),
        kind,
        cwd: cwd.into(),
        workspace: Some("/work/demo".into()),
        started,
        ended: None,
    }
}

#[test]
fn matches_by_cli_cwd_and_time_window() {
    let mut registry = PanelRegistry::default();
    registry.start(session("terminal-1", CliKind::Claude, "/work/demo", 1_000));
    registry.start(session("terminal-2", CliKind::Codex, "/work/demo", 1_000));
    registry.start(session(
        "terminal-3",
        CliKind::Claude,
        "/work/demo/sub",
        1_000,
    ));

    let at = |kind, cwd, t| registry.attribute(kind, Some(cwd), t);
    assert_eq!(
        at(CliKind::Claude, "/work/demo", 5_000).0.as_deref(),
        Some("terminal-1")
    );
    assert_eq!(
        at(CliKind::Codex, "/work/demo/", 5_000).0.as_deref(),
        Some("terminal-2")
    );
    assert_eq!(
        at(CliKind::Claude, "/work/demo/sub", 5_000).0.as_deref(),
        Some("terminal-3")
    );
    assert_eq!(
        at(CliKind::Claude, "/work/demo", 500).0,
        None,
        "before the process started"
    );
    assert_eq!(
        at(CliKind::Gemini, "/work/demo", 5_000).0,
        None,
        "another CLI"
    );

    registry.end("terminal-1", 10_000);
    let at = |kind, cwd, t| registry.attribute(kind, Some(cwd), t);
    assert_eq!(
        at(CliKind::Claude, "/work/demo", 10_000 + EXIT_GRACE_MS)
            .0
            .as_deref(),
        Some("terminal-1"),
        "a late line right after exit"
    );
    assert_eq!(
        at(CliKind::Claude, "/work/demo", 10_001 + EXIT_GRACE_MS).0,
        None
    );
}

#[test]
fn a_restart_closes_the_old_window_and_the_newest_wins() {
    let mut registry = PanelRegistry::default();
    registry.start(session("terminal-1", CliKind::Claude, "/w", 1_000));
    registry.start(session("terminal-1", CliKind::Claude, "/w", 5_000));
    registry.start(session("terminal-9", CliKind::Claude, "/w", 6_000));
    assert_eq!(
        registry
            .attribute(CliKind::Claude, Some("/w"), 7_000)
            .0
            .as_deref(),
        Some("terminal-9")
    );
    assert_eq!(
        registry
            .attribute(CliKind::Claude, Some("/w"), 5_500)
            .0
            .as_deref(),
        Some("terminal-1")
    );
}

#[test]
fn unmatched_usage_counts_under_its_workspace() {
    let mut registry = PanelRegistry::default();
    registry.note_workspace("/work/demo");
    registry.note_workspace("/work");
    let (panel, workspace) = registry.attribute(CliKind::Codex, Some("/work/demo/src/./x"), 1);
    assert_eq!(panel, None);
    assert_eq!(
        workspace.as_deref(),
        Some("/work/demo"),
        "the deepest workspace"
    );
    let (_, outside) = registry.attribute(CliKind::Codex, Some("/elsewhere/proj"), 1);
    assert_eq!(
        outside.as_deref(),
        Some(normalize("/elsewhere/proj").as_str())
    );
    assert_eq!(registry.attribute(CliKind::Codex, None, 1), (None, None));
}
