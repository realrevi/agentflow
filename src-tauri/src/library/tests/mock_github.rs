//! The mock GitHub most library tests share: three repositories (agents, a skill with a
//! script and a binary file, a Cursor rule) and AgentFlow's registry naming them.

use serde_json::json;
use tauri::async_runtime::block_on;

use super::*;
use crate::library::model::{Catalog, Item};

pub const SKILL_PATH: &str = "skills/pdf-forms/SKILL.md";

pub fn sources() -> serde_json::Value {
    json!([
        { "id": "acme-agents", "name": "Acme agents", "repo": "acme/agents", "branch": "main",
          "license": "MIT", "entries": [{ "format": "claudeSubagent", "include": ["agents/*.md"] }] },
        { "id": "acme-skills", "name": "Acme skills", "repo": "acme/skills", "license": "per-item",
          "entries": [{ "format": "claudeSkill", "include": ["skills/*/SKILL.md"],
                        "exclude": ["skills/secret/**"], "targets": ["claudeCode", "codex"] }] },
        { "id": "acme-rules", "name": "Acme rules", "repo": "acme/rules", "license": "CC0-1.0",
          "entries": [{ "format": "cursorRule", "include": ["rules/*.mdc"] }] }
    ])
}

pub fn agents_repo() -> Repo {
    Repo::new(
        "acme/agents",
        "a000000000000000000000000000000000000001",
        &[
            ("agents/code-reviewer.md", REVIEWER.as_bytes()),
            ("agents/quick-setup.md", RISKY.as_bytes()),
            ("agents/notes.md", NO_FRONTMATTER.as_bytes()),
            ("README.md", b"# Acme agents"),
        ],
    )
}

pub fn skills_repo(commit: &str, reference: &str) -> Repo {
    Repo::new(
        "acme/skills",
        commit,
        &[
            (SKILL_PATH, SKILL.as_bytes()),
            ("skills/pdf-forms/LICENSE.txt", SKILL_LICENSE.as_bytes()),
            ("skills/pdf-forms/scripts/fill.py", SKILL_SCRIPT.as_bytes()),
            ("skills/pdf-forms/reference.md", reference.as_bytes()),
            ("skills/pdf-forms/logo.png", SKILL_LOGO),
            ("skills/secret/SKILL.md", SKILL.as_bytes()),
        ],
    )
}

pub fn rules_repo() -> Repo {
    Repo::new(
        "acme/rules",
        "c000000000000000000000000000000000000003",
        &[("rules/react.mdc", RULE.as_bytes())],
    )
}

/// A mock GitHub with the three repositories and the registry.
pub fn github() -> Server {
    let server = Server::start();
    server.route(
        "/registry.json",
        vec![Reply::ok(registry(sources())).header("etag", "\"reg-1\"")],
    );
    agents_repo().serve(&server, 50);
    skills_repo("b000000000000000000000000000000000000002", SKILL_REFERENCE).serve(&server, 50);
    rules_repo().serve(&server, 50);
    server
}

pub fn refresh(library: &Library, force: bool) -> Catalog {
    block_on(library.refresh(http(), None, &[], force)).expect("refresh")
}

pub fn item<'a>(catalog: &'a Catalog, id: &str) -> &'a Item {
    catalog
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap_or_else(|| {
            panic!(
                "no item {id} in {:?}",
                catalog.items.iter().map(|i| &i.id).collect::<Vec<_>>()
            )
        })
}
