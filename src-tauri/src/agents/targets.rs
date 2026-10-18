//! One rendered template (instructions + placeholder values) -> the exact files of each tool
//! (docs/agent-targets.md). The role goes into the tool's own agent file; shared instruction
//! files (CLAUDE.md, AGENTS.md, GEMINI.md) only get a managed section pointing to it.

use serde::{Deserialize, Serialize};

use super::skills::{self, attribution};
use super::template::{Template, TemplateKind};
use super::text::{toml_multiline, toml_string, yaml_list, yaml_scalar};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Target {
    ClaudeCode,
    Codex,
    Gemini,
    Cursor,
    Agentflow,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "camelCase")]
pub enum Scope {
    /// Inside the workspace.
    #[default]
    Project,
    /// The documented folders in the home directory.
    User,
}

impl Target {
    /// Whether the tool documents a file-based user (home) scope.
    pub fn has_user_scope(self) -> bool {
        matches!(self, Self::ClaudeCode | Self::Codex | Self::Gemini)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Content {
    /// A whole file the installer owns.
    File { text: String },
    /// A managed section inside a shared file.
    Section { body: String },
    /// A whole binary file (a skill's image or font), base64.
    Binary { base64: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub target: Target,
    /// `/`-separated, relative to the workspace (project) or the home folder (user).
    pub path: String,
    pub content: Content,
}

/// What the renderers need: the template and its instructions with placeholders filled.
pub struct Rendered<'a> {
    pub template: &'a Template,
    pub instructions: String,
}

fn frontmatter(fields: &[(&str, Option<String>)], body: &str) -> String {
    let mut out = String::from("---\n");
    for (key, value) in fields {
        if let Some(value) = value {
            out.push_str(&format!("{key}: {value}\n"));
        }
    }
    out.push_str("---\n\n");
    out.push_str(body.trim());
    out.push('\n');
    out
}

/// The short section shared instruction files get.
fn pointer(template: &Template, how: &str) -> String {
    format!(
        "## {}\n\n{} {how}",
        template.name.trim(),
        template.description.trim()
    )
}

/// The tools a template can be installed for (skills only where skill folders exist,
/// commands only for Gemini CLI).
pub fn supports(template: &Template, target: Target) -> bool {
    match template.kind {
        TemplateKind::Skill => skills::skill_folder(target).is_some(),
        TemplateKind::Command => target == Target::Gemini,
        TemplateKind::Agent | TemplateKind::Rule => true,
    }
}

pub fn render(rendered: &Rendered, target: Target, scope: Scope) -> Vec<Output> {
    let t = rendered.template;
    match t.kind {
        TemplateKind::Skill => return skills::render(rendered, target),
        TemplateKind::Command => return skills::render_command(rendered, target),
        TemplateKind::Agent | TemplateKind::Rule => {}
    }
    let id = &t.id;
    let body = rendered.instructions.trim();
    // Library templates keep their source and license (frontmatter, or a TOML comment).
    let source = attribution(t).map(|s| yaml_scalar(&s));
    let user = scope == Scope::User;
    let out = |path: String, content: Content| Output {
        target,
        path,
        content,
    };
    match target {
        Target::ClaudeCode => {
            let options = t.targets.claude_code.clone().unwrap_or_default();
            let file = frontmatter(
                &[
                    ("name", Some(id.clone())),
                    ("description", Some(yaml_scalar(&t.description))),
                    ("tools", options.tools.map(|tools| tools.join(", "))),
                    ("model", options.model.map(|m| yaml_scalar(&m))),
                    ("source", source.clone()),
                ],
                body,
            );
            let how = format!("Delegate matching tasks to the `{id}` subagent.");
            vec![
                out(
                    format!(".claude/agents/{id}.md"),
                    Content::File { text: file },
                ),
                out(
                    if user {
                        ".claude/CLAUDE.md"
                    } else {
                        "CLAUDE.md"
                    }
                    .into(),
                    Content::Section {
                        body: pointer(t, &how),
                    },
                ),
            ]
        }
        Target::Codex => {
            let comment = attribution(t)
                .map(|s| format!("# Source: {}\n", s.replace('\n', " ")))
                .unwrap_or_default();
            let file = format!(
                "{comment}name = {}\ndescription = {}\n{}developer_instructions = {}\n",
                toml_string(id),
                toml_string(t.description.trim()),
                codex_options(t),
                toml_multiline(body)
            );
            let how =
                format!("Ask for the `{id}` agent (.codex/agents/{id}.toml) for these tasks.");
            vec![
                out(
                    format!(".codex/agents/{id}.toml"),
                    Content::File { text: file },
                ),
                out(
                    if user {
                        ".codex/AGENTS.md"
                    } else {
                        "AGENTS.md"
                    }
                    .into(),
                    Content::Section {
                        body: pointer(t, &how),
                    },
                ),
            ]
        }
        Target::Gemini => {
            let file = frontmatter(
                &[
                    ("name", Some(id.clone())),
                    ("description", Some(yaml_scalar(&t.description))),
                    ("source", source.clone()),
                ],
                body,
            );
            let how = format!("Delegate matching tasks to the `{id}` subagent (or `@{id}`).");
            vec![
                out(
                    format!(".gemini/agents/{id}.md"),
                    Content::File { text: file },
                ),
                out(
                    if user {
                        ".gemini/GEMINI.md"
                    } else {
                        "GEMINI.md"
                    }
                    .into(),
                    Content::Section {
                        body: pointer(t, &how),
                    },
                ),
            ]
        }
        Target::Cursor => {
            let options = t.targets.cursor.clone().unwrap_or_default();
            let globs = (!options.globs.is_empty()).then(|| options.globs.join(", "));
            let file = frontmatter(
                &[
                    ("description", Some(yaml_scalar(&t.description))),
                    ("globs", globs),
                    ("alwaysApply", Some(options.always_apply.to_string())),
                    ("source", source.clone()),
                ],
                body,
            );
            vec![out(
                format!(".cursor/rules/{id}.mdc"),
                Content::File { text: file },
            )]
        }
        Target::Agentflow => {
            let options = t.targets.agentflow.clone().unwrap_or_default();
            let mode = options.permission_mode.map(|m| {
                serde_json::to_value(m)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default()
            });
            let file = frontmatter(
                &[
                    ("name", Some(yaml_scalar(&t.name))),
                    ("description", Some(yaml_scalar(&t.description))),
                    ("provider", options.provider.map(|p| yaml_scalar(&p))),
                    ("model", options.model.map(|m| yaml_scalar(&m))),
                    ("permissionMode", mode),
                    ("tools", options.tools.map(|tools| yaml_list(&tools))),
                    ("character", t.character.as_ref().map(|c| yaml_scalar(c))),
                    (
                        "template",
                        Some(yaml_scalar(&format!("{id}@{}", t.version))),
                    ),
                    ("source", source.clone()),
                ],
                body,
            );
            vec![out(
                format!(".agentflow/agents/{id}.md"),
                Content::File { text: file },
            )]
        }
    }
}

/// Codex agent options as TOML lines (`model`, `model_reasoning_effort`, `sandbox_mode`).
fn codex_options(template: &Template) -> String {
    let Some(options) = &template.targets.codex else {
        return String::new();
    };
    [
        ("model", &options.model),
        ("model_reasoning_effort", &options.reasoning_effort),
        ("sandbox_mode", &options.sandbox_mode),
    ]
    .into_iter()
    .filter_map(|(key, value)| {
        value
            .as_ref()
            .map(|v| format!("{key} = {}\n", toml_string(v)))
    })
    .collect()
}
