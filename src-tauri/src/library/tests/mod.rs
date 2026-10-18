//! Library tests: importers on fixture files, the frontmatter and TOML readers, refreshes
//! against a local mock GitHub (ETags, commit shas, rate limits, offline, registry
//! fallback, update detection) and skill installs through the phase 8 registry. Tests
//! never reach GitHub.

mod importers;
mod install;
mod mock_github;
mod parsers;
mod refresh;
mod safety;
mod server;

use std::time::Duration;

use serde_json::{json, Value};

use super::github::Endpoints;
use super::import::{import, Imported, Input, Unconvertible};
use super::model::Format;
use super::Library;
use crate::agents::hash_bytes;
use crate::providers::Http;
use crate::util::test_dir::TestDir;
use server::{Reply, Server};

pub const REVIEWER: &str = include_str!("fixtures/code-reviewer.md");
pub const SKILL: &str = include_str!("fixtures/skills/pdf-forms/SKILL.md");
pub const SKILL_SCRIPT: &str = include_str!("fixtures/skills/pdf-forms/scripts/fill.py");
pub const SKILL_REFERENCE: &str = include_str!("fixtures/skills/pdf-forms/reference.md");
pub const SKILL_LICENSE: &str = include_str!("fixtures/skills/pdf-forms/LICENSE.txt");
pub const SKILL_LOGO: &[u8] = include_bytes!("fixtures/skills/pdf-forms/logo.png");
pub const RULE: &str = include_str!("fixtures/react.mdc");
pub const CODEX: &str = include_str!("fixtures/api-designer.toml");
pub const COMMAND: &str = include_str!("fixtures/commands/git/commit.toml");
pub const GEMINI_AGENT: &str = include_str!("fixtures/doc-writer.md");
pub const AGENTS_MD: &str = include_str!("fixtures/AGENTS.md");
pub const RISKY: &str = include_str!("fixtures/risky.md");
pub const NO_FRONTMATTER: &str = include_str!("fixtures/no-frontmatter.md");

pub const COMMIT: &str = "1111111111111111111111111111111111111111";

pub fn run(format: Format, path: &str, text: &str) -> Result<Imported, Unconvertible> {
    run_licensed(format, path, text, "MIT", None)
}

pub fn run_licensed(
    format: Format,
    path: &str,
    text: &str,
    license: &str,
    item_license: Option<&str>,
) -> Result<Imported, Unconvertible> {
    import(&Input {
        format,
        path,
        text,
        repo: "acme/agents",
        commit: COMMIT,
        sha: "abc1234",
        license,
        item_license,
        verified: true,
    })
}

/// A git-like blob sha for test content.
pub fn sha(bytes: &[u8]) -> String {
    hash_bytes(bytes).chars().take(40).collect()
}

/// A fake repository served by the mock: meta, branch head, tree and raw files.
pub struct Repo {
    pub name: &'static str,
    pub commit: String,
    pub files: Vec<(String, Vec<u8>)>,
}

impl Repo {
    pub fn new(name: &'static str, commit: &str, files: &[(&str, &[u8])]) -> Self {
        Self {
            name,
            commit: commit.into(),
            files: files
                .iter()
                .map(|(p, b)| ((*p).to_owned(), b.to_vec()))
                .collect(),
        }
    }

    pub fn tree(&self) -> Value {
        let entries: Vec<Value> = self
            .files
            .iter()
            .map(|(path, bytes)| json!({ "path": path, "type": "blob", "sha": sha(bytes), "size": bytes.len() }))
            .collect();
        json!({ "sha": format!("t{}", &self.commit[1..]), "tree": entries, "truncated": false })
    }

    /// Serves the repository (remaining requests in the rate limit: `remaining`).
    pub fn serve(&self, server: &Server, remaining: u64) {
        let reset = 4_102_444_800; // 2100-01-01
        let repo = self.name;
        let tree_sha = format!("t{}", &self.commit[1..]);
        server.route(
            &format!("/api/repos/{repo}"),
            vec![Reply::ok(
                json!({ "stargazers_count": 42, "default_branch": "main", "license": { "spdx_id": "MIT" } })
                    .to_string(),
            )
            .header("etag", &format!("\"meta-{}\"", self.commit))
            .rate(remaining, reset)],
        );
        server.route(
            &format!("/api/repos/{repo}/branches/main"),
            vec![Reply::ok(
                json!({ "commit": { "sha": self.commit, "commit": {
                    "tree": { "sha": tree_sha },
                    "committer": { "date": "2026-10-01T12:00:00Z" } } } })
                .to_string(),
            )
            .header("etag", &format!("\"head-{}\"", self.commit))
            .rate(remaining, reset)],
        );
        server.route(
            &format!("/api/repos/{repo}/git/trees/{tree_sha}?recursive=1"),
            vec![Reply::ok(self.tree().to_string()).rate(remaining, reset)],
        );
        for (path, bytes) in &self.files {
            server.route(
                &format!("/raw/{repo}/{}/{path}", self.commit),
                vec![Reply::ok(bytes.clone())],
            );
        }
    }
}

pub fn registry(sources: Value) -> String {
    json!({ "version": 1, "sources": sources }).to_string()
}

pub fn endpoints(server: &Server) -> Endpoints {
    Endpoints {
        api: format!("{}/api", server.base),
        raw: format!("{}/raw", server.base),
        registry: format!("{}/registry.json", server.base),
    }
}

pub fn library(dir: &TestDir, server: &Server) -> Library {
    Library::with_endpoints(dir.path().join("library"), endpoints(server))
}

pub fn http() -> Http {
    Http::for_tests(Duration::from_secs(5))
}
