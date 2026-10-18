//! Importers: one library file (plus, for skills, its folder) -> an installer template.
//! The original text is kept as written (literal `{{` escaped so it is never taken for a
//! placeholder); frontmatter maps to per-target options; source, license, author and commit
//! go into the template. Anything that cannot be converted is an `Unconvertible` with a
//! reason id, never dropped.

mod markdown;
mod skill;
mod text;
mod toml_formats;

use super::model::Format;
use crate::agents::{self, Target, Template, TemplateKind, TemplateSource};

pub use skill::{license_of, LICENSE_FILES};
pub use text::{category, cut, first_paragraph, humanize, one_line, parent, slug, stem};

const MAX_DESCRIPTION: usize = 500;
const MAX_NAME: usize = 80;

/// What the importer gets for one item.
pub struct Input<'a> {
    pub format: Format,
    /// The main file's path in the repository and its text.
    pub path: &'a str,
    pub text: &'a str,
    pub repo: &'a str,
    pub commit: &'a str,
    pub sha: &'a str,
    /// The source license (`per-item` = read `item_license`).
    pub license: &'a str,
    /// The text of the item's own LICENSE file, if it has one.
    pub item_license: Option<&'a str>,
    pub verified: bool,
}

pub struct Imported {
    pub template: Template,
    pub category: Option<String>,
    pub native: Vec<Target>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unconvertible {
    pub reason: &'static str,
    pub detail: String,
}

impl Unconvertible {
    pub fn new(reason: &'static str, detail: impl Into<String>) -> Self {
        Self {
            reason,
            detail: detail.into(),
        }
    }
}

/// The parts every importer fills; `finish` builds and validates the template.
#[derive(Default)]
pub(super) struct Draft {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub body: String,
    pub kind: TemplateKind,
    pub targets: agents::TargetOptions,
    pub category: Option<String>,
}

pub fn import(input: &Input) -> Result<Imported, Unconvertible> {
    let draft = match input.format {
        Format::ClaudeSubagent | Format::GeminiAgent => markdown::subagent(input)?,
        Format::CursorRule => markdown::cursor_rule(input)?,
        Format::AgentsMd => markdown::agents_md(input)?,
        Format::ClaudeSkill => skill::skill(input)?,
        Format::CodexAgent => toml_formats::codex_agent(input)?,
        Format::GeminiCommand => toml_formats::gemini_command(input)?,
    };
    finish(input, draft)
}

/// The format of a file in a custom source, from its path (and frontmatter for Markdown);
/// `None` = not an item (docs, images, a skill's own files).
pub fn guess(path: &str, text: Option<&str>) -> Option<Format> {
    let lower = path.to_ascii_lowercase();
    let name = lower.rsplit('/').next().unwrap_or(&lower);
    if name == "skill.md" {
        return Some(Format::ClaudeSkill);
    }
    if name.ends_with(".mdc") || lower.contains(".cursor/rules/") {
        return Some(Format::CursorRule);
    }
    if name.ends_with(".toml") {
        let command = lower.contains("commands/") || text.is_some_and(|t| t.contains("prompt"));
        return Some(
            if command && !text.is_some_and(|t| t.contains("developer_instructions")) {
                Format::GeminiCommand
            } else {
                Format::CodexAgent
            },
        );
    }
    if !name.ends_with(".md") {
        return None;
    }
    if ["agents.md", "claude.md", "gemini.md"].contains(&name) {
        return Some(Format::AgentsMd);
    }
    const DOCS: [&str; 7] = [
        "readme.md",
        "changelog.md",
        "contributing.md",
        "license.md",
        "code_of_conduct.md",
        "security.md",
        "claude-skills.md",
    ];
    if DOCS.contains(&name) {
        return None;
    }
    if lower.contains(".gemini/agents/") {
        return Some(Format::GeminiAgent);
    }
    Some(Format::ClaudeSubagent)
}

/// The tools a format belongs to (preselected in the wizard).
pub fn native(format: Format) -> Vec<Target> {
    match format {
        Format::ClaudeSubagent | Format::ClaudeSkill => vec![Target::ClaudeCode],
        Format::CursorRule => vec![Target::Cursor],
        Format::CodexAgent => vec![Target::Codex],
        Format::GeminiCommand | Format::GeminiAgent => vec![Target::Gemini],
        Format::AgentsMd => vec![Target::Agentflow],
    }
}

fn finish(input: &Input, draft: Draft) -> Result<Imported, Unconvertible> {
    let body = draft.body.trim();
    if body.is_empty() {
        return Err(Unconvertible::new("empty", "no instructions"));
    }
    if body.contains("<!-- agentflow:") {
        return Err(Unconvertible::new(
            "invalid",
            "contains an AgentFlow section marker",
        ));
    }
    let license = resolve_license(input, draft.license.as_deref())?;
    let id = slug(&draft.id);
    let name = draft
        .name
        .filter(|n| {
            n.chars().count() <= MAX_NAME && n.contains(|c: char| c.is_uppercase() || c == ' ')
        })
        .unwrap_or_else(|| humanize(&id));
    let description = draft
        .description
        .map(|d| one_line(&d))
        .filter(|d| !d.is_empty())
        .or_else(|| first_paragraph(body))
        .unwrap_or_else(|| name.clone());
    let owner = input.repo.split('/').next().unwrap_or(input.repo);
    let mut tags: Vec<String> = draft
        .tags
        .iter()
        .map(|t| slug(t))
        .filter(|t| t.len() >= 2)
        .collect();
    if let Some(category) = &draft.category {
        tags.insert(0, slug(category));
    }
    tags.dedup();
    tags.retain(|t| t.len() <= 32);
    tags.truncate(agents::MAX_TAGS);
    let template = Template {
        schema: 1,
        id,
        name: cut(&name, MAX_NAME),
        description: cut(&description, MAX_DESCRIPTION),
        version: "1.0.0".into(),
        author: draft.author.unwrap_or_else(|| owner.to_owned()),
        license: license.clone(),
        tags,
        character: None,
        language: "en".into(),
        instructions: agents::escape_placeholders(body),
        questions: Vec::new(),
        targets: draft.targets,
        kind: draft.kind,
        files: Vec::new(),
        source: Some(TemplateSource {
            repo: input.repo.to_owned(),
            path: input.path.to_owned(),
            commit: input.commit.to_owned(),
            sha: input.sha.to_owned(),
            license,
            verified: input.verified,
            format: input.format.id().to_owned(),
        }),
    };
    let errors = agents::validate(&template);
    if !errors.is_empty() {
        let reason = if errors.iter().any(|e| e.starts_with("instructions")) {
            "tooLarge"
        } else {
            "invalid"
        };
        return Err(Unconvertible::new(reason, errors.join("; ")));
    }
    Ok(Imported {
        template,
        category: draft.category,
        native: native(input.format),
    })
}

/// Licenses the library imports: OSI-approved (and public-domain dedications).
const OPEN: &[&str] = &[
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "MPL-2.0",
    "0BSD",
    "Unlicense",
    "CC0-1.0",
    "Zlib",
];

fn resolve_license(input: &Input, declared: Option<&str>) -> Result<String, Unconvertible> {
    if input.license != "per-item" && !input.license.trim().is_empty() {
        return Ok(input.license.to_owned());
    }
    let found = input.item_license.and_then(license_of).or_else(|| {
        declared
            .and_then(|d| OPEN.iter().find(|l| l.eq_ignore_ascii_case(d.trim())))
            .map(|l| (*l).to_owned())
    });
    match found {
        Some(license) if OPEN.contains(&license.as_str()) => Ok(license),
        Some(other) => Err(Unconvertible::new("license", other)),
        None => Err(Unconvertible::new(
            "license",
            declared.unwrap_or("no license file").to_owned(),
        )),
    }
}
