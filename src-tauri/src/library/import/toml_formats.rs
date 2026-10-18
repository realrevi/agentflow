//! TOML formats: Codex CLI subagents (`name`, `description`, `developer_instructions`,
//! `model`, `model_reasoning_effort`, `sandbox_mode`) and Gemini CLI custom commands
//! (`description`, `prompt`; the command name is the path under `commands/`).

use super::{category, stem, Draft, Input, Unconvertible};
use crate::agents::{CodexOptions, TargetOptions, TemplateKind};
use crate::library::tomlish;

fn table(input: &Input) -> Result<tomlish::Table, Unconvertible> {
    tomlish::parse(input.text).map_err(|err| Unconvertible::new("invalid", err))
}

pub fn codex_agent(input: &Input) -> Result<Draft, Unconvertible> {
    let table = table(input)?;
    let body = table.text("developer_instructions").ok_or_else(|| {
        Unconvertible::new("unknownFormat", "no developer_instructions in the TOML")
    })?;
    let name = table
        .text("name")
        .unwrap_or_else(|| stem(input.path).to_owned());
    let options = CodexOptions {
        model: table.text("model"),
        reasoning_effort: table.text("model_reasoning_effort"),
        sandbox_mode: table.text("sandbox_mode"),
    };
    let has_options = options.model.is_some()
        || options.reasoning_effort.is_some()
        || options.sandbox_mode.is_some();
    Ok(Draft {
        id: name.clone(),
        name: Some(name),
        description: table.text("description"),
        author: None,
        license: None,
        tags: Vec::new(),
        body,
        kind: TemplateKind::Agent,
        targets: TargetOptions {
            codex: has_options.then_some(options),
            ..TargetOptions::default()
        },
        category: category(input.path),
    })
}

pub fn gemini_command(input: &Input) -> Result<Draft, Unconvertible> {
    let table = table(input)?;
    let body = table
        .text("prompt")
        .ok_or_else(|| Unconvertible::new("unknownFormat", "no prompt in the TOML"))?;
    // `.gemini/commands/git/commit.toml` is `/git:commit`; installed as `git-commit`.
    let path = input.path;
    let under = path
        .find("commands/")
        .map_or(path, |i| &path[i + "commands/".len()..]);
    let name = under
        .strip_suffix(".toml")
        .unwrap_or(under)
        .replace('/', "-");
    Ok(Draft {
        id: name.clone(),
        name: None,
        description: table.text("description"),
        author: None,
        license: None,
        tags: Vec::new(),
        body,
        kind: TemplateKind::Command,
        targets: TargetOptions::default(),
        category: under.split_once('/').map(|(group, _)| group.to_owned()),
    })
}
