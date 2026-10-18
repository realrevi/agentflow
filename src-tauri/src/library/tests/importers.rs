//! Each importer on its fixture: what goes where in the template, the original text kept,
//! the attribution, and the reasons for files that cannot be converted.

use super::*;
use crate::agents::{Target, TemplateKind};
use crate::library::import::guess;

/// What the installer will write: the escaped instructions rendered back.
fn unescaped(text: &str) -> String {
    text.replace("\\{{", "{{")
}

#[test]
fn claude_subagent_maps_frontmatter_and_keeps_the_text() {
    let imported = run(
        Format::ClaudeSubagent,
        "categories/02-quality/code-reviewer.md",
        REVIEWER,
    )
    .expect("import");
    let t = &imported.template;
    assert_eq!(t.id, "code-reviewer");
    assert_eq!(t.name, "Code Reviewer");
    assert_eq!(
        t.description,
        "Expert code reviewer. Use after every change: checks style and bugs."
    );
    assert_eq!(t.author, "Jane Doe");
    assert_eq!(t.kind, TemplateKind::Agent);
    let claude = t.targets.claude_code.as_ref().expect("claude options");
    assert_eq!(
        claude.tools.as_deref(),
        Some(&["Read".to_owned(), "Grep".into(), "Glob".into()][..])
    );
    assert_eq!(claude.model.as_deref(), Some("sonnet"));
    assert!(
        t.instructions.contains("\\{{ bugs }}"),
        "literal braces escaped"
    );
    assert_eq!(
        unescaped(&t.instructions),
        "You are a senior code reviewer.\n\nCheck the diff for {{ bugs }} and report them by severity."
    );
    let source = t.source.as_ref().expect("source");
    assert_eq!(
        (source.repo.as_str(), source.commit.as_str()),
        ("acme/agents", COMMIT)
    );
    assert_eq!(
        (source.license.as_str(), source.format.as_str()),
        ("MIT", "claudeSubagent")
    );
    assert_eq!(imported.category.as_deref(), Some("quality"));
    assert_eq!(t.tags, vec!["quality"]);
    assert_eq!(imported.native, vec![Target::ClaudeCode]);
    assert!(crate::agents::validate(t).is_empty());
}

#[test]
fn skill_reads_skill_md_and_its_per_item_license() {
    let imported = run_licensed(
        Format::ClaudeSkill,
        "skills/pdf-forms/SKILL.md",
        SKILL,
        "per-item",
        Some(SKILL_LICENSE),
    )
    .expect("import");
    let t = &imported.template;
    assert_eq!(t.kind, TemplateKind::Skill);
    assert_eq!(t.id, "pdf-forms");
    assert_eq!(
        t.description,
        "Fills PDF forms from structured data. Use when the user asks to fill or inspect a PDF form."
    );
    assert_eq!(t.license, "Apache-2.0");
    assert_eq!(t.author, "Acme Docs Team");
    let tools = t.targets.claude_code.as_ref().and_then(|c| c.tools.clone());
    assert_eq!(
        tools,
        Some(vec!["Read".to_owned(), "Bash(python:*)".into()])
    );
    assert!(t.instructions.starts_with("# PDF forms\n\n1. Inspect"));

    // No OSI license: listed as unsupported, with what was found.
    let proprietary = "Copyright Acme. All rights reserved.";
    let err = run_licensed(
        Format::ClaudeSkill,
        "skills/x/SKILL.md",
        SKILL,
        "per-item",
        Some(proprietary),
    )
    .err()
    .expect("refused");
    assert_eq!(
        (err.reason, err.detail.as_str()),
        ("license", "proprietary")
    );
    let err = run_licensed(
        Format::ClaudeSkill,
        "skills/x/SKILL.md",
        SKILL,
        "per-item",
        None,
    )
    .err()
    .expect("refused");
    assert_eq!(err.reason, "license");
}

#[test]
fn cursor_rule_maps_globs_and_always_apply() {
    let imported = run(Format::CursorRule, "rules/react.mdc", RULE).expect("import");
    let t = &imported.template;
    assert_eq!((t.id.as_str(), t.name.as_str()), ("react", "React"));
    assert_eq!(t.kind, TemplateKind::Rule);
    assert_eq!(t.description, "React component rules");
    let cursor = t.targets.cursor.as_ref().expect("cursor options");
    assert_eq!(cursor.globs, vec!["**/*.tsx", "**/*.jsx"]);
    assert!(!cursor.always_apply);
    assert!(t
        .instructions
        .starts_with("# React\n\n- Use function components"));
    assert_eq!(imported.native, vec![Target::Cursor]);
}

#[test]
fn codex_agent_maps_its_options() {
    let imported = run(
        Format::CodexAgent,
        "categories/01-api/api-designer.toml",
        CODEX,
    )
    .expect("import");
    let t = &imported.template;
    assert_eq!(t.id, "api-designer");
    assert_eq!(t.description, "Designs REST and GraphQL APIs.");
    let codex = t.targets.codex.as_ref().expect("codex options");
    assert_eq!(codex.model.as_deref(), Some("gpt-5.3-codex"));
    assert_eq!(codex.reasoning_effort.as_deref(), Some("high"));
    assert_eq!(codex.sandbox_mode.as_deref(), Some("read-only"));
    assert_eq!(
        unescaped(&t.instructions),
        "You design APIs.\nQuote \"names\" and keep \\ backslashes; template syntax like {{name}} stays."
    );
    assert_eq!(imported.native, vec![Target::Codex]);
}

#[test]
fn gemini_command_keeps_its_own_placeholders() {
    let imported = run(
        Format::GeminiCommand,
        ".gemini/commands/git/commit.toml",
        COMMAND,
    )
    .expect("import");
    let t = &imported.template;
    assert_eq!(
        (t.id.as_str(), t.kind),
        ("git-commit", TemplateKind::Command)
    );
    assert_eq!(
        t.description,
        "Writes a commit message for the staged changes"
    );
    assert!(unescaped(&t.instructions).contains("!{git diff --staged}\n\nExtra notes: {{args}}"));
    assert_eq!(imported.category.as_deref(), Some("git"));
    assert_eq!(imported.native, vec![Target::Gemini]);
    assert!(crate::agents::validate(t).is_empty());
}

#[test]
fn gemini_agent_and_agents_md() {
    let gemini = run(
        Format::GeminiAgent,
        ".gemini/agents/doc-writer.md",
        GEMINI_AGENT,
    )
    .expect("import");
    assert_eq!(gemini.template.id, "doc-writer");
    assert!(
        gemini.template.targets.claude_code.is_none(),
        "Gemini tool names stay out"
    );
    assert_eq!(gemini.native, vec![Target::Gemini]);

    let prompt = run(Format::AgentsMd, "release/AGENTS.md", AGENTS_MD).expect("import");
    let t = &prompt.template;
    assert_eq!(
        (t.id.as_str(), t.name.as_str()),
        ("release-helper", "Release Helper")
    );
    assert_eq!(
        t.description,
        "Prepares releases: changelog, version bump and tag."
    );
    assert_eq!(t.instructions, AGENTS_MD.trim());
    assert_eq!(prompt.native, vec![Target::Agentflow]);
}

#[test]
fn unconvertible_files_say_why() {
    let reason = |format, path, text| run(format, path, text).err().expect("refused").reason;
    assert_eq!(
        reason(Format::ClaudeSubagent, "a/notes.md", NO_FRONTMATTER),
        "noFrontmatter"
    );
    assert_eq!(
        reason(Format::ClaudeSubagent, "a/x.md", "---\nname: x\n"),
        "frontmatter"
    );
    assert_eq!(
        reason(Format::ClaudeSubagent, "a/x.md", "---\nname: x\n---\n\n"),
        "empty"
    );
    assert_eq!(
        reason(Format::CodexAgent, "a/x.toml", "name = \"x\"\n"),
        "unknownFormat"
    );
    assert_eq!(
        reason(Format::GeminiCommand, "a/x.toml", "prompt = \"\"\"\nx\n"),
        "invalid"
    );
    let huge = format!("---\nname: x\n---\n{}", "a".repeat(100_001));
    assert_eq!(reason(Format::ClaudeSubagent, "a/x.md", &huge), "tooLarge");
    let marker = "---\nname: x\n---\n<!-- agentflow:begin x -->";
    assert_eq!(reason(Format::ClaudeSubagent, "a/x.md", marker), "invalid");
}

#[test]
fn custom_sources_guess_the_format_per_file() {
    assert_eq!(
        guess("a/skills/x/SKILL.md", None),
        Some(Format::ClaudeSkill)
    );
    assert_eq!(guess("rules/react.mdc", None), Some(Format::CursorRule));
    assert_eq!(
        guess(".gemini/commands/git/commit.toml", None),
        Some(Format::GeminiCommand)
    );
    assert_eq!(
        guess("agents/api.toml", Some(CODEX)),
        Some(Format::CodexAgent)
    );
    assert_eq!(
        guess("x/.gemini/agents/doc.md", None),
        Some(Format::GeminiAgent)
    );
    assert_eq!(guess("AGENTS.md", None), Some(Format::AgentsMd));
    assert_eq!(
        guess("agents/reviewer.md", None),
        Some(Format::ClaudeSubagent)
    );
    assert_eq!(guess("README.md", None), None);
    assert_eq!(guess("assets/logo.png", None), None);
}

#[test]
fn ids_names_and_descriptions_are_made_valid() {
    use crate::library::import::{humanize, slug};
    assert_eq!(slug("Next.js App Router!"), "next-js-app-router");
    assert_eq!(slug("x"), "item-x");
    assert_eq!(slug(&"a".repeat(80)).len(), 48);
    assert_eq!(humanize("api_designer-v2"), "Api Designer V2");
    let long = format!(
        "---\nname: x-y\ndescription: {}\n---\nBody",
        "word ".repeat(200)
    );
    let t = run(Format::ClaudeSubagent, "a/x.md", &long)
        .expect("import")
        .template;
    assert_eq!(t.description.chars().count(), 500);
    assert!(t.description.ends_with('…'));
}
