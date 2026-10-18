//! Agent Skills (Claude Code, Codex, Gemini CLI, Cursor): a folder with `SKILL.md`
//! (frontmatter `name`, `description`, optional `license`, `allowed-tools`, `metadata`) and
//! other files. The importer reads SKILL.md; the other files join the template at install
//! (`library::detail`). Per-item licenses come from the folder's LICENSE file.

use super::{category, parent, Draft, Input, Unconvertible};
use crate::agents::{ClaudeOptions, TargetOptions, TemplateKind};
use crate::library::frontmatter;

/// A skill folder's own license file names, in the order they are looked for.
pub const LICENSE_FILES: [&str; 4] = ["LICENSE.txt", "LICENSE", "LICENSE.md", "license.txt"];

pub fn skill(input: &Input) -> Result<Draft, Unconvertible> {
    let doc =
        frontmatter::parse(input.text).map_err(|err| Unconvertible::new("frontmatter", err))?;
    if !doc.has_frontmatter {
        return Err(Unconvertible::new(
            "noFrontmatter",
            "SKILL.md needs name and description frontmatter",
        ));
    }
    let folder = parent(input.path).rsplit('/').next().unwrap_or("skill");
    let name = doc.text("name").unwrap_or_else(|| folder.to_owned());
    let tools = doc.list("allowed-tools");
    let tools: Vec<String> = if tools.len() == 1 {
        // `allowed-tools: Read Grep Bash(git:*)` is space-separated in the spec.
        tools[0].split_whitespace().map(str::to_owned).collect()
    } else {
        tools
    };
    let targets = TargetOptions {
        claude_code: (!tools.is_empty()).then_some(ClaudeOptions {
            tools: Some(tools),
            model: None,
        }),
        ..TargetOptions::default()
    };
    Ok(Draft {
        id: name.clone(),
        name: Some(name),
        description: doc.text("description"),
        author: doc
            .nested("metadata", "author")
            .or_else(|| doc.text("author")),
        license: doc.text("license"),
        tags: Vec::new(),
        body: doc.body,
        kind: TemplateKind::Skill,
        targets,
        category: category(input.path),
    })
}

/// The SPDX id of a license text, for the common open licenses.
pub fn license_of(text: &str) -> Option<String> {
    let head: String = text
        .chars()
        .take(3000)
        .collect::<String>()
        .to_ascii_lowercase();
    let id = if head.contains("apache license") && head.contains("version 2.0") {
        "Apache-2.0"
    } else if head.contains("permission is hereby granted, free of charge") {
        "MIT"
    } else if head.contains("mozilla public license") && head.contains("2.0") {
        "MPL-2.0"
    } else if head.contains("redistribution and use in source and binary forms") {
        if head.contains("neither the name") || head.contains("contributors may be used") {
            "BSD-3-Clause"
        } else {
            "BSD-2-Clause"
        }
    } else if head.contains("permission to use, copy, modify, and/or distribute") {
        "ISC"
    } else if head.contains("this is free and unencumbered software") {
        "Unlicense"
    } else if head.contains("cc0 1.0") || head.contains("creative commons zero") {
        "CC0-1.0"
    } else if head.contains("all rights reserved") || head.contains("proprietary") {
        "proprietary"
    } else {
        return Some("unknown".into());
    };
    Some(id.into())
}
