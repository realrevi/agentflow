//! Library template kinds (phase 10): skill folders (text, script and binary files) through
//! plan, install, update detection and uninstall with the registry; Gemini commands; Codex
//! options; the literal-brace escape; unsafe skill file paths.

use std::path::Path;

use serde_json::json;

use crate::agents::apply::{install, uninstall};
use crate::agents::placeholders::escape;
use crate::agents::plan::{plan, Confirmations, InstallRequest, Status};
use crate::agents::registry::Registry;
use crate::agents::roots::Roots;
use crate::agents::targets::{Scope, Target};
use crate::agents::template::Template;
use crate::agents::validate::validate;
use crate::error::AppResult;
use crate::util::base64;
use crate::util::test_dir::TestDir;

fn roots(dir: &TestDir) -> Roots {
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).expect("work");
    std::fs::create_dir_all(dir.path().join("home")).expect("home");
    Roots::new(&work, Some(dir.path().join("home"))).expect("roots")
}

fn trash(path: &Path) -> AppResult<()> {
    std::fs::remove_file(path)?;
    Ok(())
}

fn skill() -> Template {
    serde_json::from_value(json!({
        "schema": 1,
        "id": "pdf-helper",
        "name": "PDF Helper",
        "description": "Works with PDF files.",
        "version": "1.0.0",
        "license": "Apache-2.0",
        "kind": "skill",
        "instructions": escape("# PDF\n\nRun `scripts/fill.py {{input}}` to fill forms."),
        "files": [
            { "path": "scripts/fill.py", "content": "print('fill')\n" },
            { "path": "reference/forms.md", "content": "# Forms\n" },
            { "path": "assets/logo.png", "content": base64::encode(&[0x89, b'P', b'N', b'G', 0, 1]), "binary": true }
        ],
        "source": {
            "repo": "acme/skills", "path": "skills/pdf-helper", "commit": "0123456789abcdef0123456789abcdef01234567",
            "sha": "abc", "license": "Apache-2.0", "verified": true, "format": "claudeSkill"
        }
    }))
    .expect("skill template")
}

fn request(template: Template, targets: &[Target]) -> InstallRequest {
    InstallRequest {
        template,
        answers: Default::default(),
        targets: targets.to_vec(),
        scope: Scope::Project,
    }
}

#[test]
fn a_skill_folder_installs_whole_and_uninstalls_clean() {
    let dir = TestDir::new("skill-install");
    let roots = roots(&dir);
    let work = roots.workspace.path().to_path_buf();
    let req = request(skill(), &[Target::ClaudeCode, Target::Codex]);
    let planned = plan(&roots, &Registry::default(), &req).expect("plan");
    let paths: Vec<&str> = planned.files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&".claude/skills/pdf-helper/SKILL.md"));
    assert!(paths.contains(&".claude/skills/pdf-helper/scripts/fill.py"));
    assert!(paths.contains(&".agents/skills/pdf-helper/assets/logo.png"));
    assert!(!paths
        .iter()
        .any(|p| p.ends_with("CLAUDE.md") || p.ends_with("AGENTS.md")));
    let logo = planned
        .files
        .iter()
        .find(|f| f.path.ends_with("logo.png"))
        .expect("logo");
    assert_eq!(logo.after, "(binary file, 6 bytes)");

    install(&roots, &req, &Confirmations::default(), trash).expect("install");
    let skill_md =
        std::fs::read_to_string(work.join(".claude/skills/pdf-helper/SKILL.md")).expect("md");
    assert!(skill_md.starts_with("---\nname: pdf-helper\n"));
    assert!(skill_md.contains("license: Apache-2.0"));
    assert!(skill_md.contains("source: acme/skills@0123456 (Apache-2.0)"));
    assert!(
        skill_md.contains("Run `scripts/fill.py {{input}}` to fill forms."),
        "escaped braces come back as written"
    );
    assert_eq!(
        std::fs::read(work.join(".agents/skills/pdf-helper/assets/logo.png")).expect("png"),
        vec![0x89, b'P', b'N', b'G', 0, 1]
    );
    let registry = Registry::load(&work);
    let installed = registry.get("pdf-helper").expect("recorded");
    assert_eq!(installed.files.len(), 8);
    assert_eq!(
        installed
            .template
            .source
            .as_ref()
            .map(|s| s.commit.as_str()),
        Some("0123456789abcdef0123456789abcdef01234567")
    );

    // Installing again changes nothing; an edited binary is a conflict.
    let again = plan(&roots, &registry, &req).expect("plan");
    assert!(again.files.iter().all(|f| f.status == Status::Unchanged));
    std::fs::write(work.join(".claude/skills/pdf-helper/assets/logo.png"), b"x").expect("edit");
    let edited = plan(&roots, &registry, &req).expect("plan");
    assert!(edited
        .files
        .iter()
        .any(|f| f.path.ends_with("logo.png") && f.status == Status::Conflict));

    let confirm = Confirmations {
        overwrite: vec![".claude/skills/pdf-helper/assets/logo.png".into()],
        user_scope: false,
    };
    uninstall(&roots, "pdf-helper", &confirm, &[], trash).expect("uninstall");
    assert!(
        !work.join(".claude/skills/pdf-helper").exists(),
        "empty folders pruned"
    );
    assert!(!work.join(".agents/skills/pdf-helper").exists());
    assert!(
        work.join(".claude/skills").exists(),
        "the skills folder itself stays"
    );
    assert!(Registry::load(&work).get("pdf-helper").is_none());
}

#[test]
fn skills_only_go_to_tools_with_skill_folders_and_commands_only_to_gemini() {
    let dir = TestDir::new("skill-targets");
    let roots = roots(&dir);
    let err = plan(
        &roots,
        &Registry::default(),
        &request(skill(), &[Target::Agentflow]),
    )
    .expect_err("no skills in AgentFlow");
    assert!(err.to_string().contains("skill"));

    let mut command = skill();
    command.kind = crate::agents::template::TemplateKind::Command;
    command.files.clear();
    command.id = "pr-review".into();
    command.instructions = escape("Review the PR {{args}} and !{git diff}.");
    let planned = plan(
        &roots,
        &Registry::default(),
        &request(command.clone(), &[Target::Gemini]),
    )
    .expect("plan");
    assert_eq!(planned.files.len(), 1);
    let toml = &planned.files[0].after;
    assert_eq!(planned.files[0].path, ".gemini/commands/pr-review.toml");
    assert!(toml.starts_with("# Source: acme/skills@0123456 (Apache-2.0)\n"));
    assert!(toml.contains("{{args}}"), "{toml}");
    assert!(plan(
        &roots,
        &Registry::default(),
        &request(command, &[Target::ClaudeCode])
    )
    .is_err());
}

#[test]
fn codex_options_and_attribution_render() {
    let dir = TestDir::new("codex-options");
    let roots = roots(&dir);
    let mut agent = skill();
    agent.kind = crate::agents::template::TemplateKind::Agent;
    agent.files.clear();
    agent.targets.codex = Some(crate::agents::template::CodexOptions {
        model: Some("gpt-5".into()),
        reasoning_effort: Some("high".into()),
        sandbox_mode: Some("read-only".into()),
    });
    let planned = plan(
        &roots,
        &Registry::default(),
        &request(agent, &[Target::Codex, Target::ClaudeCode]),
    )
    .expect("plan");
    let codex = planned
        .files
        .iter()
        .find(|f| f.path == ".codex/agents/pdf-helper.toml")
        .expect("codex file");
    assert!(codex.after.contains("model = \"gpt-5\"\n"));
    assert!(codex.after.contains("model_reasoning_effort = \"high\"\n"));
    assert!(codex.after.contains("sandbox_mode = \"read-only\"\n"));
    let claude = planned
        .files
        .iter()
        .find(|f| f.path == ".claude/agents/pdf-helper.md")
        .expect("claude file");
    assert!(claude
        .after
        .contains("source: acme/skills@0123456 (Apache-2.0)"));
}

#[test]
fn unsafe_skill_paths_and_misplaced_files_fail_validation() {
    for bad in ["../evil.sh", "/etc/passwd", r"..\..\x", "C:/x", "SKILL.md"] {
        let mut template = skill();
        template.files[0].path = bad.into();
        assert!(!validate(&template).is_empty(), "{bad}");
    }
    let mut agent = skill();
    agent.kind = crate::agents::template::TemplateKind::Agent;
    assert!(validate(&agent)
        .iter()
        .any(|e| e.contains("only skills have extra files")));
    assert!(validate(&skill()).is_empty(), "{:?}", validate(&skill()));
}

#[test]
fn user_scope_skills_stay_inside_the_documented_folders() {
    let dir = TestDir::new("skill-user");
    let roots = roots(&dir);
    assert!(roots
        .resolve(Scope::User, ".claude/skills/pdf/scripts/a.py")
        .is_ok());
    assert!(roots
        .resolve(Scope::User, ".claude/skills/pdf/../../x")
        .is_err());
    assert!(roots.resolve(Scope::User, ".claude/skills/a").is_err());
    assert!(roots
        .resolve(Scope::User, ".gemini/commands/x.toml")
        .is_ok());
}
