//! Template validation: the rules of schemas/agent-template.schema.json, plus what JSON
//! Schema cannot say (unique question ids, placeholders that name a question, defaults that
//! fit their type). Errors are plain sentences with the field path, for the wizard and for the
//! "Create with AI" retry.

use std::collections::HashSet;

use regex::Regex;
use serde_json::Value;

use super::detect::DETECTORS;
use super::placeholders::{placeholders, BUILT_INS};
use super::relpath::safe_relative;
use super::section::MARKER_PREFIX;
use super::template::{QuestionKind, Template, TemplateKind, SCHEMA_VERSION};
use crate::agent::tools::TOOL_NAMES;

/// Library items keep their original text, which can be long (a skill's SKILL.md).
pub const MAX_INSTRUCTIONS: usize = 100_000;
pub const MAX_FILES: usize = 200;
/// All of a skill's files together (base64 counts as stored).
pub const MAX_FILES_BYTES: usize = 6 * 1024 * 1024;
pub const MAX_QUESTIONS: usize = 30;
pub const MAX_TAGS: usize = 10;

fn matches(pattern: &str, value: &str) -> bool {
    Regex::new(pattern).is_ok_and(|re| re.is_match(value))
}

pub const ID_PATTERN: &str = r"^[a-z0-9][a-z0-9-]{1,47}$";
pub const QUESTION_ID_PATTERN: &str = r"^[a-z][a-zA-Z0-9_]{0,39}$";
pub const VERSION_PATTERN: &str = r"^\d+\.\d+\.\d+$";
pub const TAG_PATTERN: &str = r"^[a-z0-9][a-z0-9-]{0,31}$";

/// Every problem found (empty = valid).
pub fn validate(template: &Template) -> Vec<String> {
    let mut errors = Vec::new();
    let mut check = |ok: bool, message: String| {
        if !ok {
            errors.push(message);
        }
    };
    check(
        template.schema == SCHEMA_VERSION,
        format!("schema: must be {SCHEMA_VERSION}"),
    );
    check(
        matches(ID_PATTERN, &template.id),
        "id: 2-48 lowercase letters, digits or hyphens, starting with a letter or digit".into(),
    );
    check(
        (1..=80).contains(&template.name.trim().chars().count()),
        "name: 1-80 characters".into(),
    );
    check(
        (1..=500).contains(&template.description.trim().chars().count())
            && !template.description.contains('\n'),
        "description: one line, 1-500 characters".into(),
    );
    check(
        matches(VERSION_PATTERN, &template.version),
        "version: major.minor.patch, e.g. 1.0.0".into(),
    );
    check(
        template.tags.len() <= MAX_TAGS && template.tags.iter().all(|t| matches(TAG_PATTERN, t)),
        format!("tags: at most {MAX_TAGS}, lowercase words with hyphens"),
    );
    check(
        matches(r"^[a-zA-Z]{2,3}(-[a-zA-Z0-9]{2,8})*$", &template.language),
        "language: a language code such as en or pt-BR".into(),
    );
    let length = template.instructions.chars().count();
    check(
        length > 0 && length <= MAX_INSTRUCTIONS,
        format!("instructions: 1-{MAX_INSTRUCTIONS} characters"),
    );
    check(
        !template.instructions.contains(MARKER_PREFIX),
        format!("instructions: must not contain \"{MARKER_PREFIX}\""),
    );
    check(
        template.questions.len() <= MAX_QUESTIONS,
        format!("questions: at most {MAX_QUESTIONS}"),
    );

    let mut ids = HashSet::new();
    for (index, question) in template.questions.iter().enumerate() {
        let at = format!("questions[{index}] ({})", question.id);
        check(
            matches(QUESTION_ID_PATTERN, &question.id),
            format!("{at}.id: a letter, then letters, digits or _ (max 40)"),
        );
        check(
            !BUILT_INS.contains(&question.id.as_str()),
            format!("{at}.id: \"{}\" is a built-in placeholder", question.id),
        );
        check(
            ids.insert(question.id.clone()),
            format!("{at}.id: used twice"),
        );
        check(
            (1..=120).contains(&question.label.trim().chars().count()),
            format!("{at}.label: 1-120 characters"),
        );
        let choices = matches!(
            question.kind,
            QuestionKind::Choice | QuestionKind::MultiChoice
        );
        check(
            !choices || !question.options.is_empty(),
            format!("{at}.options: needed for choice questions"),
        );
        if let Some(detect) = &question.detect {
            check(
                DETECTORS.contains(&detect.as_str()),
                format!(
                    "{at}.detect: unknown detector \"{detect}\" (known: {})",
                    DETECTORS.join(", ")
                ),
            );
        }
        if let Some(default) = &question.default {
            check(
                fits(question.kind, default, &question.options),
                format!("{at}.default: does not fit the question type"),
            );
        }
    }
    for name in placeholders(&template.instructions) {
        check(
            ids.contains(&name) || BUILT_INS.contains(&name.as_str()),
            format!("instructions: unknown placeholder {{{{{name}}}}}"),
        );
    }
    if let Some(agentflow) = &template.targets.agentflow {
        for tool in agentflow.tools.iter().flatten() {
            check(
                TOOL_NAMES.contains(&tool.as_str()),
                format!(
                    "targets.agentflow.tools: unknown tool \"{tool}\" (known: {})",
                    TOOL_NAMES.join(", ")
                ),
            );
        }
    }
    if let Some(source) = &template.source {
        check(
            matches(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$", &source.repo),
            "source.repo: owner/name".into(),
        );
        check(
            source.commit.is_empty() || matches(r"^[0-9a-f]{7,64}$", &source.commit),
            "source.commit: a commit sha".into(),
        );
        check(
            !source.license.trim().is_empty() && source.license.len() <= 64,
            "source.license: the repository's license".into(),
        );
    }
    if let Some(cursor) = &template.targets.cursor {
        check(
            cursor
                .globs
                .iter()
                .all(|g| !g.trim().is_empty() && !g.contains('\n')),
            "targets.cursor.globs: non-empty, one line each".into(),
        );
    }
    check_files(template, &mut errors);
    errors
}

/// Skill files: only skills have them; safe, unique paths; size caps.
fn check_files(template: &Template, errors: &mut Vec<String>) {
    if template.files.is_empty() {
        return;
    }
    if template.kind != TemplateKind::Skill {
        errors.push("files: only skills have extra files".into());
        return;
    }
    if template.files.len() > MAX_FILES {
        errors.push(format!("files: at most {MAX_FILES}"));
    }
    let total: usize = template.files.iter().map(|f| f.content.len()).sum();
    if total > MAX_FILES_BYTES {
        errors.push(format!(
            "files: at most {} MB together",
            MAX_FILES_BYTES / 1024 / 1024
        ));
    }
    let mut seen = HashSet::new();
    for file in &template.files {
        match safe_relative(&file.path) {
            Ok(path) if path.eq_ignore_ascii_case("SKILL.md") => {
                errors.push("files: SKILL.md comes from the instructions".into());
            }
            Ok(path) if !seen.insert(path.to_ascii_lowercase()) => {
                errors.push(format!("files: {path} twice"));
            }
            Ok(_) => {}
            Err(err) => errors.push(format!("files: {err}")),
        }
    }
}

/// Whether `value` is an answer (or default) of the right shape.
pub fn fits(kind: QuestionKind, value: &Value, options: &[String]) -> bool {
    match (kind, value) {
        (QuestionKind::Boolean, Value::Bool(_)) => true,
        (QuestionKind::MultiChoice, Value::Array(items)) => items.iter().all(|item| {
            item.as_str()
                .is_some_and(|s| options.iter().any(|o| o == s))
        }),
        (QuestionKind::Choice, Value::String(s)) => s.is_empty() || options.iter().any(|o| o == s),
        (
            QuestionKind::Text | QuestionKind::Multiline | QuestionKind::Color | QuestionKind::Font,
            Value::String(s),
        ) => !s.contains(MARKER_PREFIX),
        _ => false,
    }
}
