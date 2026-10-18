//! From the library to the workspace: the detail view downloads a skill's files, scans
//! them, and its template installs and uninstalls through the phase 8 registry.

use std::path::Path;

use super::mock_github::{github, refresh};
use super::*;
use crate::agents::{self, Confirmations, InstallRequest, Registry, Roots, Scope, Target};
use crate::error::AppResult;
use tauri::async_runtime::block_on;

fn trash(path: &Path) -> AppResult<()> {
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn a_library_skill_installs_with_all_its_files_and_uninstalls() {
    let (dir, server) = (TestDir::new("lib-install"), github());
    let library = library(&dir, &server);
    refresh(&library, true);
    let id = "acme-skills:skills/pdf-forms/SKILL.md";
    let detail = block_on(library.detail(http(), None, id)).expect("detail");
    assert_eq!(detail.problem, None);
    let files: Vec<(&str, bool, bool)> = detail
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.binary, f.executable))
        .collect();
    assert_eq!(
        files,
        vec![
            ("LICENSE.txt", false, false),
            ("scripts/fill.py", false, true),
            ("reference.md", false, false),
            ("logo.png", true, false),
        ]
    );
    assert!(detail
        .flags
        .iter()
        .any(|f| f.id == "executable" && f.excerpt == "scripts/fill.py"));
    assert!(detail.text.starts_with("---\nname: pdf-forms"));
    let template = detail.template.expect("template");
    assert_eq!(template.files.len(), 4);

    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).expect("work");
    let roots = Roots::new(&work, None).expect("roots");
    let request = InstallRequest {
        template,
        answers: Default::default(),
        targets: vec![Target::ClaudeCode],
        scope: Scope::Project,
    };
    agents::install(&roots, &request, &Confirmations::default(), trash).expect("install");
    let skill = work.join(".claude/skills/pdf-forms");
    assert_eq!(
        std::fs::read_to_string(skill.join("scripts/fill.py")).expect("script"),
        SKILL_SCRIPT
    );
    assert_eq!(
        std::fs::read(skill.join("logo.png")).expect("logo"),
        SKILL_LOGO
    );
    let md = std::fs::read_to_string(skill.join("SKILL.md")).expect("SKILL.md");
    assert!(md.contains("license: Apache-2.0"));
    assert!(md.contains("source: acme/skills@b000000 (Apache-2.0)"));
    assert!(md.contains("allowed-tools: [Read, Bash(python:*)]") || md.contains("allowed-tools:"));
    let registry = Registry::load(&work);
    let installed = registry.get("pdf-forms").expect("recorded");
    let source = installed.template.source.as_ref().expect("source");
    assert_eq!(source.commit, "b000000000000000000000000000000000000002");
    assert!(source.verified);

    agents::uninstall(&roots, "pdf-forms", &Confirmations::default(), &[], trash)
        .expect("uninstall");
    assert!(!skill.exists());
    assert!(Registry::load(&work).get("pdf-forms").is_none());
}

#[test]
fn a_flagged_item_carries_its_warnings_into_the_detail() {
    let (dir, server) = (TestDir::new("lib-flags"), github());
    let library = library(&dir, &server);
    refresh(&library, true);
    let detail = block_on(library.detail(http(), None, "acme-agents:agents/quick-setup.md"))
        .expect("detail");
    let ids: Vec<&str> = detail.flags.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, vec!["downloadExec", "disablePermissions"]);
    assert!(detail.flags[0].excerpt.contains("curl -fsSL"));
    // The template is still there: flags warn, the preview shows everything.
    assert!(detail.template.is_some());
}
