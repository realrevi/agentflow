//! Refreshes against the mock GitHub: imports per source, the blob cache and ETags (a
//! second refresh downloads nothing), the daily limit, offline mode, rate limits, the
//! registry fallback, update detection by sha and custom sources.

use super::mock_github::{github, item, refresh, skills_repo, sources, SKILL_PATH};
use super::*;
use crate::settings::CustomSource;
use tauri::async_runtime::block_on;

#[test]
fn imports_every_source_and_lists_what_it_cannot() {
    let (dir, server) = (TestDir::new("lib-refresh"), github());
    let library = library(&dir, &server);
    let catalog = refresh(&library, true);
    assert_eq!(catalog.registry, "remote");
    let counts: Vec<(&str, usize, usize)> = catalog
        .sources
        .iter()
        .map(|s| (s.id.as_str(), s.imported, s.unsupported))
        .collect();
    assert_eq!(
        counts,
        vec![
            ("acme-agents", 2, 1),
            ("acme-skills", 1, 0),
            ("acme-rules", 1, 0)
        ]
    );
    let notes = &catalog.unsupported[0];
    assert_eq!(
        (notes.path.as_str(), notes.reason.as_str()),
        ("agents/notes.md", "noFrontmatter")
    );

    let reviewer = item(&catalog, "acme-agents:agents/code-reviewer.md");
    assert!(reviewer.verified && reviewer.flags.is_empty());
    assert_eq!(reviewer.stars, 42);
    assert_eq!(reviewer.updated_at.as_deref(), Some("2026-10-01T12:00:00Z"));
    assert!(reviewer.tokens > 10);
    assert_eq!(reviewer.native, vec![crate::agents::Target::ClaudeCode]);
    assert_eq!(
        reviewer.url,
        "https://github.com/acme/agents/blob/a000000000000000000000000000000000000001/agents/code-reviewer.md"
    );
    let risky = item(&catalog, "acme-agents:agents/quick-setup.md");
    let ids: Vec<&str> = risky.flags.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, vec!["downloadExec", "disablePermissions"]);

    let skill = item(&catalog, &format!("acme-skills:{SKILL_PATH}"));
    assert_eq!(skill.license, "Apache-2.0", "from the skill's LICENSE.txt");
    assert_eq!(skill.native.len(), 2, "the registry entry's targets");
    let files: Vec<&str> = skill.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        files,
        vec!["LICENSE.txt", "scripts/fill.py", "reference.md", "logo.png"]
    );
    assert_eq!(skill.flags[0].id, "executable");
    assert_eq!(skill.flags[0].excerpt, "scripts/fill.py");
    // Only main files and license files were downloaded; skill files wait for the detail.
    assert_eq!(server.count("/raw/acme/skills/"), 2);
    assert_eq!(
        server.count("/raw/acme/agents/"),
        3,
        "README.md is not an item"
    );
}

#[test]
fn a_second_refresh_uses_etags_and_the_blob_cache() {
    let (dir, server) = (TestDir::new("lib-etag"), github());
    let library = library(&dir, &server);
    let first = refresh(&library, true);
    server.clear_log();
    let second = refresh(&library, true);
    assert_eq!(second.items, first.items);
    assert_eq!(server.count("/raw/"), 0, "contents come from the cache");
    assert_eq!(
        server.count("/api/repos/acme/agents/git/trees"),
        0,
        "same commit: no tree call"
    );
    let meta = server
        .requests()
        .into_iter()
        .find(|r| r.path == "/api/repos/acme/agents")
        .expect("meta request");
    assert_eq!(
        meta.headers.get("if-none-match").map(String::as_str),
        Some("\"meta-a000000000000000000000000000000000000001\"")
    );
    let registry = server
        .requests()
        .into_iter()
        .find(|r| r.path == "/registry.json")
        .expect("registry");
    assert_eq!(
        registry.headers.get("if-none-match").map(String::as_str),
        Some("\"reg-1\"")
    );
    assert_eq!(second.registry, "cache", "304: the cached registry");

    // An automatic refresh within a day sends nothing at all.
    server.clear_log();
    let auto = refresh(&library, false);
    assert!(server.requests().is_empty());
    assert_eq!(auto.items.len(), first.items.len());
}

#[test]
fn offline_the_cached_library_still_works() {
    let dir = TestDir::new("lib-offline");
    let first = {
        let server = github();
        refresh(&library(&dir, &server), true)
    };
    // The server is gone: every request fails like an unplugged network.
    let dead = Server::start();
    let offline = library(&dir, &dead);
    drop(dead);
    let catalog = refresh(&offline, true);
    assert_eq!(catalog.items, first.items);
    assert_eq!(
        catalog.refreshed_at, first.refreshed_at,
        "a failed refresh is not a refresh"
    );
    assert!(catalog
        .sources
        .iter()
        .all(|s| s.error.as_ref().is_some_and(|e| e.kind == "network")));
    assert_eq!(offline.catalog().items.len(), first.items.len());
    let detail = block_on(offline.detail(http(), None, "acme-agents:agents/code-reviewer.md"))
        .expect("detail");
    assert!(detail.template.is_some());
    let skill = block_on(offline.detail(http(), None, &format!("acme-skills:{SKILL_PATH}")))
        .expect("detail");
    assert_eq!(
        skill.problem.as_deref(),
        Some("notCached"),
        "skill files were never downloaded"
    );
    assert!(skill.template.is_none());
}

#[test]
fn a_used_up_rate_limit_stops_the_refresh_and_says_when() {
    let (dir, server) = (TestDir::new("lib-rate"), Server::start());
    server.route("/registry.json", vec![Reply::ok(registry(sources()))]);
    server.route(
        "/api/repos/acme/agents",
        vec![
            Reply::status(403, "{\"message\":\"API rate limit exceeded\"}").rate(0, 4_102_444_800),
        ],
    );
    let library = library(&dir, &server);
    let catalog = refresh(&library, true);
    for source in &catalog.sources {
        let error = source.error.as_ref().expect("error");
        assert_eq!(error.kind, "rateLimited", "{}", source.id);
        assert_eq!(error.reset_at, Some(4_102_444_800_000));
    }
    assert_eq!(server.count("/api/repos/acme/skills"), 0, "never hammer");
    assert_eq!(catalog.rate.map(|r| r.remaining), Some(0));
    server.clear_log();
    let again = refresh(&library, true);
    assert_eq!(server.count("/api/"), 0, "no API call before the reset");
    assert!(again.sources[0]
        .error
        .as_ref()
        .is_some_and(|e| e.kind == "rateLimited"));
}

#[test]
fn the_bundled_registry_is_the_fallback() {
    let (dir, server) = (TestDir::new("lib-bundled"), Server::start());
    server.route("/registry.json", vec![Reply::ok("{\"version\": 99}")]);
    let catalog = refresh(&library(&dir, &server), true);
    assert_eq!(catalog.registry, "bundled");
    let bundled: Vec<String> = crate::library::sources::bundled()
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert!(bundled.len() >= 6);
    let ids: Vec<String> = catalog.sources.iter().map(|s| s.id.clone()).collect();
    assert_eq!(ids, bundled);
    assert!(catalog.sources.iter().all(|s| s.verified));
}

#[test]
fn an_upstream_change_shows_as_a_new_sha() {
    let (dir, server) = (TestDir::new("lib-update"), github());
    let library = library(&dir, &server);
    let before = refresh(&library, true);
    skills_repo(
        "b000000000000000000000000000000000000009",
        "# Field types\n\nNow with dates.\n",
    )
    .serve(&server, 50);
    let after = refresh(&library, true);
    let id = format!("acme-skills:{SKILL_PATH}");
    assert_ne!(
        item(&before, &id).sha,
        item(&after, &id).sha,
        "a skill file changed"
    );
    assert_eq!(
        item(&after, &id).commit,
        "b000000000000000000000000000000000000009"
    );
    let rule = "acme-rules:rules/react.mdc";
    assert_eq!(item(&before, rule).sha, item(&after, rule).sha);
}

#[test]
fn custom_sources_are_unverified_and_guessed() {
    let (dir, server) = (TestDir::new("lib-custom"), github());
    Repo::new(
        "someone/mixed",
        "d000000000000000000000000000000000000004",
        &[
            ("agents/code-reviewer.md", REVIEWER.as_bytes()),
            ("agents/react.mdc", RULE.as_bytes()),
            ("agents/README.md", b"# docs"),
            ("other/ignored.md", REVIEWER.as_bytes()),
        ],
    )
    .serve(&server, 50);
    let custom = [
        CustomSource {
            url: "https://github.com/someone/mixed/tree/main/agents".into(),
            path: String::new(),
        },
        CustomSource {
            url: "https://gitlab.com/x/y".into(),
            path: String::new(),
        },
    ];
    let library = library(&dir, &server);
    let catalog = block_on(library.refresh(http(), None, &custom, true)).expect("refresh");
    let source = catalog
        .sources
        .iter()
        .find(|s| s.repo == "someone/mixed")
        .expect("custom");
    assert!(!source.verified);
    assert_eq!((source.imported, source.license.as_str()), (2, "MIT"));
    let rule = catalog
        .items
        .iter()
        .find(|i| i.path == "agents/react.mdc")
        .expect("rule");
    assert_eq!(rule.format, Format::CursorRule);
    assert!(!rule.verified);
    let invalid = catalog.sources.last().expect("invalid");
    assert_eq!(
        invalid.error.as_ref().map(|e| e.kind.as_str()),
        Some("invalid")
    );
}
