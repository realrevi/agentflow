//! Skills (phase 10): a folder with `SKILL.md` (frontmatter `name`, `description`, optional
//! `license`, `allowed-tools`, `metadata`; the instructions as the body) and the skill's other
//! files, copied as they are. AgentFlow never runs them. Only tools that document skill
//! folders get one (docs/agent-targets.md). Also: the attribution line of library templates.

use super::roots::Roots;
use super::targets::{Content, Output, Rendered, Scope, Target};
use super::template::Template;
use super::text::{toml_multiline, toml_string, yaml_list, yaml_scalar};

/// Where a tool looks for skills (relative to the workspace, or home for user scope).
pub fn skill_folder(target: Target) -> Option<&'static str> {
    match target {
        Target::ClaudeCode => Some(".claude/skills"),
        Target::Codex => Some(".agents/skills"),
        Target::Gemini => Some(".gemini/skills"),
        Target::Cursor => Some(".cursor/skills"),
        Target::Agentflow => None,
    }
}

/// `owner/repo@abc1234 (MIT)` for templates from the library.
pub fn attribution(template: &Template) -> Option<String> {
    let source = template.source.as_ref()?;
    let commit: String = source.commit.chars().take(7).collect();
    let at = if commit.is_empty() {
        String::new()
    } else {
        format!("@{commit}")
    };
    Some(format!("{}{at} ({})", source.repo, source.license.trim()))
}

pub fn render(rendered: &Rendered, target: Target) -> Vec<Output> {
    let Some(folder) = skill_folder(target) else {
        return Vec::new();
    };
    let t = rendered.template;
    let base = format!("{folder}/{}", t.id);
    let mut head = format!(
        "---\nname: {}\ndescription: {}\n",
        t.id,
        yaml_scalar(&t.description)
    );
    if let Some(tools) = t
        .targets
        .claude_code
        .as_ref()
        .and_then(|o| o.tools.as_ref())
    {
        if target == Target::ClaudeCode {
            head.push_str(&format!("allowed-tools: {}\n", yaml_list(tools)));
        }
    }
    if !t.license.trim().is_empty() {
        head.push_str(&format!("license: {}\n", yaml_scalar(&t.license)));
    }
    if let Some(source) = attribution(t) {
        head.push_str(&format!("metadata:\n  source: {}\n", yaml_scalar(&source)));
    }
    head.push_str("---\n\n");
    head.push_str(rendered.instructions.trim());
    head.push('\n');
    let mut out = vec![Output {
        target,
        path: format!("{base}/SKILL.md"),
        content: Content::File { text: head },
    }];
    for file in &t.files {
        let content = if file.binary {
            Content::Binary {
                base64: file.content.clone(),
            }
        } else {
            Content::File {
                text: file.content.clone(),
            }
        };
        out.push(Output {
            target,
            path: format!("{base}/{}", file.path.replace('\\', "/")),
            content,
        });
    }
    out
}

/// A Gemini CLI custom command: `.gemini/commands/<id>.toml` with `description` and
/// `prompt` (its `{{args}}`, `!{...}` and `@{...}` are Gemini's, kept as they are).
pub fn render_command(rendered: &Rendered, target: Target) -> Vec<Output> {
    if target != Target::Gemini {
        return Vec::new();
    }
    let t = rendered.template;
    let comment = attribution(t)
        .map(|s| format!("# Source: {}\n", s.replace(['\n', '\r'], " ")))
        .unwrap_or_default();
    let text = format!(
        "{comment}description = {}\nprompt = {}\n",
        toml_string(t.description.trim()),
        toml_multiline(rendered.instructions.trim())
    );
    vec![Output {
        target,
        path: format!(".gemini/commands/{}.toml", t.id),
        content: Content::File { text },
    }]
}

/// After an uninstall: removes the now empty folders of a skill (`<skill folder>/<id>/...`),
/// never the skill folder itself or anything that still holds files.
pub fn prune_empty(roots: &Roots, scope: Scope, rel: &str) {
    let is_skill = [
        ".claude/skills/",
        ".agents/skills/",
        ".gemini/skills/",
        ".cursor/skills/",
    ]
    .iter()
    .any(|folder| rel.starts_with(folder));
    if !is_skill {
        return;
    }
    let mut parts: Vec<&str> = rel.split('/').collect();
    while parts.len() > 3 {
        parts.pop();
        let Ok(dir) = roots.resolve_dir(scope, &parts.join("/")) else {
            return;
        };
        let empty = std::fs::read_dir(&dir).is_ok_and(|mut entries| entries.next().is_none());
        if !empty || std::fs::remove_dir(&dir).is_err() {
            return;
        }
    }
}
