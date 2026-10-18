use std::path::Path;

use serde_json::json;

use super::sample;
use crate::agents::apply::{install, uninstall, uninstall_plan};
use crate::agents::plan::{plan, Confirmations, ConflictKind, InstallRequest, Status};
use crate::agents::profiles::parse;
use crate::agents::registry::Registry;
use crate::agents::roots::Roots;
use crate::agents::targets::{Scope, Target};
use crate::agents::template::Answers;
use crate::conversations::PermissionMode;
use crate::error::AppResult;
use crate::util::test_dir::TestDir;

struct Fixture {
    _dir: TestDir,
    roots: Roots,
}

impl Fixture {
    fn new() -> Self {
        let dir = TestDir::new("installer");
        let work = dir.path().join("work");
        let home = dir.path().join("home");
        std::fs::create_dir_all(&work).expect("work");
        std::fs::create_dir_all(&home).expect("home");
        let roots = Roots::new(&work, Some(home)).expect("roots");
        Self { _dir: dir, roots }
    }

    fn work(&self, rel: &str) -> std::path::PathBuf {
        self.roots.workspace.path().join(rel)
    }

    fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.work(rel)).expect("read")
    }

    fn write(&self, rel: &str, text: &str) {
        let path = self.work(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    }
}

fn trash(path: &Path) -> AppResult<()> {
    std::fs::remove_file(path)?;
    Ok(())
}

fn request(targets: &[Target], answers: Answers) -> InstallRequest {
    let mut template = sample();
    template.targets.agentflow = Some(crate::agents::template::AgentflowOptions {
        permission_mode: Some(PermissionMode::ReadOnly),
        tools: Some(vec!["read_file".into(), "search".into()]),
        ..Default::default()
    });
    InstallRequest {
        template,
        answers,
        targets: targets.to_vec(),
        scope: Scope::Project,
    }
}

fn colors(value: &str) -> Answers {
    let mut answers = Answers::new();
    answers.insert("colors".into(), json!(value));
    answers
}

fn none() -> Confirmations {
    Confirmations::default()
}

#[test]
fn installs_updates_and_uninstalls_without_touching_user_text() {
    let fx = Fixture::new();
    // An existing CLAUDE.md with CRLF line endings and the user's own content.
    fx.write("CLAUDE.md", "# Shop\r\nUse tabs.\r\n");
    let first = request(
        &[Target::ClaudeCode, Target::Cursor, Target::Agentflow],
        colors("red #f00"),
    );
    let planned = plan(&fx.roots, &Registry::default(), &first).expect("plan");
    let status = |path: &str| {
        planned
            .files
            .iter()
            .find(|f| f.path == path)
            .map(|f| f.status)
    };
    assert_eq!(status(".claude/agents/demo-agent.md"), Some(Status::Create));
    assert_eq!(status("CLAUDE.md"), Some(Status::Update));
    assert!(planned.files.iter().all(|f| f.conflict.is_none()));
    install(&fx.roots, &first, &none(), trash).expect("install");

    let agent = fx.read(".claude/agents/demo-agent.md");
    assert!(agent.starts_with("---\nname: demo-agent\ndescription: Checks demo things.\n"));
    assert!(agent.contains("Colors: red #f00"));
    let claude = fx.read("CLAUDE.md");
    assert!(claude.starts_with("# Shop\r\nUse tabs.\r\n"), "{claude:?}");
    assert!(claude.contains("<!-- agentflow:begin demo-agent -->\r\n"));
    let profile =
        parse("demo-agent", &fx.read(".agentflow/agents/demo-agent.md")).expect("profile");
    assert_eq!(profile.permission_mode, Some(PermissionMode::ReadOnly));
    assert_eq!(
        profile.tools,
        Some(vec!["read_file".to_owned(), "search".to_owned()])
    );
    assert!(profile.instructions.contains("Colors: red #f00"));

    // The user writes around our section; an update with new answers keeps their text.
    let edited = claude.replace(
        "<!-- agentflow:begin",
        "My sentence.\r\n<!-- agentflow:begin",
    );
    fx.write("CLAUDE.md", &format!("{edited}After our section.\r\n"));
    let second = request(
        &[Target::ClaudeCode, Target::Cursor, Target::Agentflow],
        colors("blue #00f"),
    );
    let registry = Registry::load(fx.roots.workspace.path());
    let update = plan(&fx.roots, &registry, &second).expect("update plan");
    assert!(
        update.files.iter().all(|f| f.conflict.is_none()),
        "{update:#?}"
    );
    install(&fx.roots, &second, &none(), trash).expect("update");
    assert!(fx
        .read(".agentflow/agents/demo-agent.md")
        .contains("blue #00f"));
    let claude = fx.read("CLAUDE.md");
    assert!(claude.contains("My sentence.\r\n") && claude.contains("After our section.\r\n"));

    // Uninstall: our files go, the shared file stays with the user's text only.
    uninstall(&fx.roots, "demo-agent", &none(), &[], trash).expect("uninstall");
    assert!(!fx.work(".claude/agents/demo-agent.md").exists());
    assert!(!fx.work(".cursor/rules/demo-agent.mdc").exists());
    // The blank line written before the section stays: the user's text now sits between
    // it and the section, and nothing outside the markers is removed.
    assert_eq!(
        fx.read("CLAUDE.md"),
        "# Shop\r\nUse tabs.\r\n\r\nMy sentence.\r\nAfter our section.\r\n"
    );
    assert!(Registry::load(fx.roots.workspace.path())
        .get("demo-agent")
        .is_none());
}

#[test]
fn a_shared_file_the_installer_created_goes_away_with_its_section() {
    let fx = Fixture::new();
    let req = request(&[Target::Codex], Answers::new());
    install(&fx.roots, &req, &none(), trash).expect("install");
    assert!(fx
        .read("AGENTS.md")
        .starts_with("<!-- agentflow:begin demo-agent -->"));
    assert!(fx
        .read(".codex/agents/demo-agent.toml")
        .starts_with("name = \"demo-agent\""));
    uninstall(&fx.roots, "demo-agent", &none(), &[], trash).expect("uninstall");
    assert!(!fx.work("AGENTS.md").exists());
}

#[test]
fn foreign_and_edited_files_need_a_confirmation() {
    let fx = Fixture::new();
    fx.write(".cursor/rules/demo-agent.mdc", "my own rule\n");
    let req = request(&[Target::Cursor], Answers::new());
    let planned = plan(&fx.roots, &Registry::default(), &req).expect("plan");
    assert_eq!(planned.files[0].conflict, Some(ConflictKind::Foreign));
    let err = install(&fx.roots, &req, &none(), trash).unwrap_err();
    assert_eq!(err.kind(), "conflict");
    assert_eq!(fx.read(".cursor/rules/demo-agent.mdc"), "my own rule\n");
    let confirm = Confirmations {
        overwrite: vec![".cursor/rules/demo-agent.mdc".into()],
        user_scope: false,
    };
    install(&fx.roots, &req, &confirm, trash).expect("confirmed");

    // Edited after install: update and uninstall both stop at it.
    fx.write(".cursor/rules/demo-agent.mdc", "edited by me\n");
    let registry = Registry::load(fx.roots.workspace.path());
    let again = plan(&fx.roots, &registry, &req).expect("plan");
    assert_eq!(again.files[0].conflict, Some(ConflictKind::Edited));
    let removal = uninstall_plan(&fx.roots, "demo-agent").expect("uninstall plan");
    assert_eq!(removal.files[0].status, Status::Conflict);
    assert!(uninstall(&fx.roots, "demo-agent", &none(), &[], trash).is_err());
    assert_eq!(fx.read(".cursor/rules/demo-agent.mdc"), "edited by me\n");
    // "Keep" leaves the file and forgets the install.
    let keep = vec![".cursor/rules/demo-agent.mdc".to_owned()];
    uninstall(&fx.roots, "demo-agent", &none(), &keep, trash).expect("keep");
    assert_eq!(fx.read(".cursor/rules/demo-agent.mdc"), "edited by me\n");
    assert!(Registry::load(fx.roots.workspace.path()).agents.is_empty());
}

#[test]
fn dropping_a_target_on_update_removes_its_files() {
    let fx = Fixture::new();
    let both = request(&[Target::Gemini, Target::Cursor], Answers::new());
    install(&fx.roots, &both, &none(), trash).expect("install");
    assert!(fx.work(".gemini/agents/demo-agent.md").exists());
    let only = request(&[Target::Gemini], Answers::new());
    let registry = Registry::load(fx.roots.workspace.path());
    let planned = plan(&fx.roots, &registry, &only).expect("plan");
    let cursor = planned
        .files
        .iter()
        .find(|f| f.path == ".cursor/rules/demo-agent.mdc")
        .expect("removal row");
    assert!(cursor.remove && cursor.status == Status::Remove);
    install(&fx.roots, &only, &none(), trash).expect("update");
    assert!(!fx.work(".cursor/rules/demo-agent.mdc").exists());
    assert!(fx.read("GEMINI.md").contains("agentflow:begin demo-agent"));
}

#[test]
fn user_scope_needs_its_own_confirmation_and_documented_folders() {
    let fx = Fixture::new();
    let mut req = request(&[Target::ClaudeCode], Answers::new());
    req.scope = Scope::User;
    let err = install(&fx.roots, &req, &none(), trash).unwrap_err();
    assert!(err.to_string().contains("home folder"), "{err}");
    let confirm = Confirmations {
        overwrite: Vec::new(),
        user_scope: true,
    };
    let (planned, _) = install(&fx.roots, &req, &confirm, trash).expect("user install");
    assert!(planned
        .files
        .iter()
        .any(|f| f.display == "~/.claude/agents/demo-agent.md"));
    let home = fx.roots.home.clone().expect("home");
    assert!(home.join(".claude/agents/demo-agent.md").is_file());
    assert!(home.join(".claude/CLAUDE.md").is_file());
    assert!(!fx.work(".claude").exists(), "nothing in the project");

    // No documented home folder for Cursor or AgentFlow; nothing outside the list.
    for target in [Target::Cursor, Target::Agentflow] {
        let mut req = request(&[target], Answers::new());
        req.scope = Scope::User;
        assert!(plan(&fx.roots, &Registry::default(), &req).is_err());
    }
    for rel in [
        ".ssh/config",
        ".claude/settings.json",
        ".claude/agents/a/b.md",
        "x.md",
    ] {
        assert!(fx.roots.resolve(Scope::User, rel).is_err(), "{rel}");
    }
}

#[test]
fn project_paths_cannot_leave_the_workspace() {
    let fx = Fixture::new();
    for rel in ["../outside.md", "/etc/passwd", "a/../../b.md"] {
        assert!(fx.roots.resolve(Scope::Project, rel).is_err(), "{rel}");
    }
    // A drive path is absolute only on Windows; elsewhere "C:" is just a folder name.
    #[cfg(windows)]
    assert!(fx.roots.resolve(Scope::Project, "C:/Windows/x.md").is_err());
    #[cfg(unix)]
    {
        let outside = fx
            .roots
            .workspace
            .path()
            .parent()
            .expect("parent")
            .join("elsewhere");
        std::fs::create_dir_all(&outside).expect("outside");
        std::os::unix::fs::symlink(&outside, fx.work(".claude")).expect("symlink");
        let req = request(&[Target::ClaudeCode], Answers::new());
        assert!(install(&fx.roots, &req, &none(), trash).is_err());
        assert!(!outside.join("agents/demo-agent.md").exists());
    }
}
