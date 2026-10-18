//! The built-in starter templates (English), embedded in the binary. A test checks that
//! each one is valid.

use super::template::Template;

const SOURCES: [&str; 9] = [
    include_str!("starters/code-reviewer.json"),
    include_str!("starters/ui-visual-tester.json"),
    include_str!("starters/test-writer.json"),
    include_str!("starters/bug-hunter.json"),
    include_str!("starters/refactorer.json"),
    include_str!("starters/docs-writer.json"),
    include_str!("starters/security-reviewer.json"),
    include_str!("starters/commit-pr-writer.json"),
    include_str!("starters/workstream-manager.json"),
];

pub fn starters() -> Vec<Template> {
    SOURCES
        .iter()
        .filter_map(|source| match serde_json::from_str(source) {
            Ok(template) => Some(template),
            Err(err) => {
                tracing::warn!("a built-in agent template does not parse: {err}");
                None
            }
        })
        .collect()
}
