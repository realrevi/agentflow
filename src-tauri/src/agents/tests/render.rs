//! Snapshots of the exact files two starters produce for every target. A missing snapshot
//! is written (and the test fails once) so it can be reviewed; set UPDATE_SNAPSHOTS=1 to
//! rewrite them all after an intended change.

use std::path::PathBuf;

use serde_json::json;

use crate::agents::placeholders::{render as fill, values};
use crate::agents::starters::starters;
use crate::agents::targets::{render, Content, Rendered, Scope, Target};
use crate::agents::template::Answers;

const TARGETS: [Target; 5] = [
    Target::ClaudeCode,
    Target::Codex,
    Target::Gemini,
    Target::Cursor,
    Target::Agentflow,
];

fn answers(id: &str) -> Answers {
    let mut answers = Answers::new();
    match id {
        "code-reviewer" => {
            answers.insert("languages".into(), json!("TypeScript, CSS"));
            answers.insert("styleGuide".into(), json!("Prefer early returns."));
        }
        "ui-visual-tester" => {
            answers.insert(
                "brandColors".into(),
                json!("primary #1e40af, accent #f59e0b"),
            );
            answers.insert("fonts".into(), json!("Inter, JetBrains Mono"));
            answers.insert("uiFramework".into(), json!("React + Tailwind CSS"));
            answers.insert("devCommand".into(), json!("pnpm dev"));
        }
        _ => {}
    }
    answers
}

fn snapshot_dir(id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/agents/tests/snapshots")
        .join(id)
}

#[test]
fn starters_render_the_expected_files_per_target() {
    let update = std::env::var_os("UPDATE_SNAPSHOTS").is_some();
    let mut written = Vec::new();
    for id in ["code-reviewer", "ui-visual-tester"] {
        let template = starters()
            .into_iter()
            .find(|t| t.id == id)
            .expect("starter");
        let filled = values(&template, &answers(id), "demo-shop").expect("values");
        let instructions = fill(&template.instructions, &filled).expect("render");
        let rendered = Rendered {
            template: &template,
            instructions,
        };
        for target in TARGETS {
            for output in render(&rendered, target, Scope::Project) {
                let text = match &output.content {
                    Content::File { text } => text.clone(),
                    Content::Section { body } => format!("[managed section]\n{body}\n"),
                    Content::Binary { base64 } => format!("[binary]\n{base64}\n"),
                };
                let file = snapshot_dir(id).join(output.path.replace('/', "__"));
                match std::fs::read_to_string(&file) {
                    Ok(expected) if !update => {
                        assert_eq!(text, expected.replace("\r\n", "\n"), "{}", file.display())
                    }
                    _ => {
                        std::fs::create_dir_all(file.parent().expect("dir")).expect("mkdir");
                        std::fs::write(&file, &text).expect("write snapshot");
                        written.push(file.display().to_string());
                    }
                }
            }
        }
    }
    assert!(
        update || written.is_empty(),
        "new snapshots, review them: {written:#?}"
    );
}

#[test]
fn user_scope_paths_follow_each_tool() {
    let template = starters().into_iter().next().expect("starter");
    let rendered = Rendered {
        template: &template,
        instructions: "x".into(),
    };
    let paths = |target| -> Vec<String> {
        render(&rendered, target, Scope::User)
            .into_iter()
            .map(|o| o.path)
            .collect()
    };
    assert_eq!(
        paths(Target::ClaudeCode),
        [".claude/agents/code-reviewer.md", ".claude/CLAUDE.md"]
    );
    assert_eq!(
        paths(Target::Codex),
        [".codex/agents/code-reviewer.toml", ".codex/AGENTS.md"]
    );
    assert_eq!(
        paths(Target::Gemini),
        [".gemini/agents/code-reviewer.md", ".gemini/GEMINI.md"]
    );
}

#[test]
fn generated_frontmatter_and_toml_stay_parseable() {
    use crate::agents::text::{toml_multiline, yaml_scalar};
    assert_eq!(yaml_scalar("Reviews code."), "Reviews code.");
    assert_eq!(yaml_scalar("a: b"), "\"a: b\"");
    assert_eq!(yaml_scalar("yes"), "\"yes\"");
    assert_eq!(yaml_scalar("say \"hi\"\n"), "\"say \\\"hi\\\"\"");
    // Backslashes doubled, a run of three quotes broken up, a closing line of its own.
    assert_eq!(
        toml_multiline(r#"a\b """ end""#),
        "\"\"\"\n".to_owned() + r#"a\\b ""\" end""# + "\n\"\"\""
    );
}
