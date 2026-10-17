use crate::agents::section::{content, remove, upsert};

const ID: &str = "code-reviewer";

#[test]
fn inserts_at_the_end_and_never_touches_the_rest() {
    let original = "# Project\n\nUse tabs.\n";
    let once = upsert(original, ID, "## Code Reviewer\n\nReviews code.").expect("insert");
    assert_eq!(
        once,
        "# Project\n\nUse tabs.\n\n<!-- agentflow:begin code-reviewer -->\n## Code Reviewer\n\nReviews code.\n<!-- agentflow:end code-reviewer -->\n"
    );
    assert!(once.starts_with(original));
    // Idempotent.
    assert_eq!(
        upsert(&once, ID, "## Code Reviewer\n\nReviews code.").expect("again"),
        once
    );
    assert_eq!(
        content(&once, ID).expect("content").as_deref(),
        Some("## Code Reviewer\n\nReviews code.")
    );
    // Removing restores the original exactly.
    assert_eq!(remove(&once, ID).expect("remove"), original);
    assert_eq!(remove(original, ID).expect("absent"), original);
    // An empty file gets just the section; removing it leaves nothing.
    let fresh = upsert("", ID, "x").expect("fresh");
    assert_eq!(remove(&fresh, ID).expect("remove"), "");
}

#[test]
fn updates_in_place_between_user_text() {
    let file = "intro\n<!-- agentflow:begin code-reviewer -->\nold\n<!-- agentflow:end code-reviewer -->\nMy own sentence after.\n";
    let updated = upsert(file, ID, "new").expect("update");
    assert_eq!(
        updated,
        "intro\n<!-- agentflow:begin code-reviewer -->\nnew\n<!-- agentflow:end code-reviewer -->\nMy own sentence after.\n"
    );
    let removed = remove(&updated, ID).expect("remove");
    assert_eq!(removed, "intro\nMy own sentence after.\n");
    // Another agent's section is left alone.
    let two = upsert(&updated, "other", "theirs").expect("second id");
    assert_eq!(remove(&two, "other").expect("remove other"), updated);
}

#[test]
fn keeps_crlf_line_endings() {
    let original = "# Title\r\nKeep me.\r\n";
    let inserted = upsert(original, ID, "line one\nline two").expect("insert");
    assert!(inserted.starts_with(original));
    assert!(
        !inserted.replace("\r\n", "").contains('\n'),
        "only CRLF: {inserted:?}"
    );
    assert_eq!(
        content(&inserted, ID).expect("content").as_deref(),
        Some("line one\nline two")
    );
    assert_eq!(remove(&inserted, ID).expect("remove"), original);
}

#[test]
fn broken_markers_are_left_for_the_user() {
    let missing_end = "x\n<!-- agentflow:begin code-reviewer -->\nhalf\n";
    assert!(upsert(missing_end, ID, "y").is_err());
    assert!(remove(missing_end, ID).is_err());
    let twice = "<!-- agentflow:begin code-reviewer -->\n<!-- agentflow:begin code-reviewer -->\n<!-- agentflow:end code-reviewer -->\n";
    assert!(upsert(twice, ID, "y").is_err());
    assert!(upsert("<!-- agentflow:end code-reviewer -->\n", ID, "y").is_err());
}
