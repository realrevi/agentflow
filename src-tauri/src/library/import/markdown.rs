//! Markdown formats: Claude Code and Gemini CLI subagents (frontmatter `name`,
//! `description`, `tools`, `model`), Cursor rules (`.mdc`: `description`, `globs`,
//! `alwaysApply`) and AGENTS.md-style prompts (no frontmatter needed).

use super::{category, parent, stem, Draft, Input, Unconvertible};
use crate::agents::{ClaudeOptions, CursorOptions, TargetOptions, TemplateKind};
use crate::library::frontmatter::{self, Doc};
use crate::library::model::Format;

fn doc(input: &Input) -> Result<Doc, Unconvertible> {
    frontmatter::parse(input.text).map_err(|err| Unconvertible::new("frontmatter", err))
}

fn author(doc: &Doc) -> Option<String> {
    doc.text("author")
        .or_else(|| doc.nested("metadata", "author"))
}

fn tags(doc: &Doc) -> Vec<String> {
    let mut tags = doc.list("tags");
    if tags.is_empty() {
        tags = doc.get("metadata").map_or_else(Vec::new, |_| {
            doc.nested("metadata", "tags")
                .map(|t| t.split(',').map(|s| s.trim().to_owned()).collect())
                .unwrap_or_default()
        });
    }
    tags
}

pub fn subagent(input: &Input) -> Result<Draft, Unconvertible> {
    let doc = doc(input)?;
    if !doc.has_frontmatter {
        return Err(Unconvertible::new(
            "noFrontmatter",
            "a subagent needs name and description frontmatter",
        ));
    }
    let name = doc
        .text("name")
        .ok_or_else(|| Unconvertible::new("noFrontmatter", "no name in the frontmatter"))?;
    let mut targets = TargetOptions::default();
    if input.format == Format::ClaudeSubagent {
        let tools = doc.list("tools");
        let model = doc.text("model");
        if !tools.is_empty() || model.is_some() {
            targets.claude_code = Some(ClaudeOptions {
                tools: (!tools.is_empty()).then_some(tools),
                model,
            });
        }
    }
    Ok(Draft {
        id: name.clone(),
        name: Some(name),
        description: doc.text("description"),
        author: author(&doc),
        license: doc.text("license"),
        tags: tags(&doc),
        body: doc.body.clone(),
        kind: TemplateKind::Agent,
        targets,
        category: category(input.path),
    })
}

pub fn cursor_rule(input: &Input) -> Result<Draft, Unconvertible> {
    let doc = doc(input)?;
    let globs: Vec<String> = doc
        .list("globs")
        .into_iter()
        .filter(|g| !g.contains('\n'))
        .collect();
    let always_apply = doc
        .text("alwaysApply")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    let id = stem(input.path).to_owned();
    Ok(Draft {
        name: None,
        description: doc.text("description"),
        author: author(&doc),
        license: None,
        tags: tags(&doc),
        body: doc.body.clone(),
        kind: TemplateKind::Rule,
        targets: TargetOptions {
            cursor: Some(CursorOptions {
                globs,
                always_apply,
            }),
            ..TargetOptions::default()
        },
        category: category(input.path),
        id,
    })
}

/// A whole Markdown prompt: the name from its first heading (or the folder), the
/// description from the first paragraph.
pub fn agents_md(input: &Input) -> Result<Draft, Unconvertible> {
    let doc = doc(input)?;
    let heading = doc
        .body
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .map(|h| h.trim().to_owned())
        .filter(|h| !h.is_empty());
    let folder = parent(input.path).rsplit('/').next().unwrap_or("");
    let fallback = if folder.is_empty() {
        input.repo.rsplit('/').next().unwrap_or("agent").to_owned()
    } else {
        folder.to_owned()
    };
    let id = doc
        .text("name")
        .or_else(|| heading.clone())
        .unwrap_or_else(|| fallback.clone());
    Ok(Draft {
        name: doc.text("name").or(heading),
        description: doc.text("description"),
        author: author(&doc),
        license: doc.text("license"),
        tags: tags(&doc),
        body: doc.body.clone(),
        kind: TemplateKind::Agent,
        targets: TargetOptions::default(),
        category: category(input.path),
        id,
    })
}
