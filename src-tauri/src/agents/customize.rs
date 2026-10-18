//! "Customize with AI" for library templates: the model never rewrites the original text.
//! It only adds project-specific questions and a "Project specifics" section that uses
//! them as placeholders; both are merged into a copy of the template, validated (an
//! invalid reply gets the usual retry) and reviewed by the user in the wizard.

use serde::Deserialize;

use super::template::{Question, Template};
use super::validate::{validate, MAX_QUESTIONS};

/// The original instructions shown to the model (enough to see what the item does).
const MAX_CONTEXT_CHARS: usize = 12_000;
const MAX_NEW_QUESTIONS: usize = 6;
pub const SECTION: &str = "## Project specifics";

pub fn system(language: &str) -> String {
    format!(
        "You adapt an existing AI coding agent template from a community library to the \
         user's project for AgentFlow. Do NOT rewrite, translate or repeat the original \
         instructions. Reply with ONE JSON object and nothing else (no code fences):\n\
         {{\"questions\": [...], \"projectSpecifics\": \"...\"}}\n\n\
         - `questions`: 1-{MAX_NEW_QUESTIONS} questions whose answers make the agent fit a \
         project (stack, commands, conventions, paths). Each: {{\"id\": \"camelCase\", \
         \"label\": \"...\", \"type\": \"text|multiline|choice|multiChoice|boolean|color|font\", \
         \"default\": ..., \"options\": [...] (choice/multiChoice only), \"detect\": \
         \"languages|packageManager|testFramework|testCommand|uiFramework|colors|fonts|spacing|breakpoints\" \
         (optional, when the workspace can answer it)}}. No other keys. Do not reuse \
         existing question ids.\n\
         - `projectSpecifics`: short Markdown (no heading) telling the agent how to apply \
         the answers, using {{{{questionId}}}} placeholders of your questions only.\n\
         - Write labels and projectSpecifics in language \"{language}\"."
    )
}

pub fn prompt(base: &Template, notes: &str) -> String {
    let text: String = base.instructions.chars().take(MAX_CONTEXT_CHARS).collect();
    let existing: Vec<&str> = base.questions.iter().map(|q| q.id.as_str()).collect();
    let notes = if notes.is_empty() {
        "(none: suggest the most useful project questions)"
    } else {
        notes
    };
    format!(
        "Template: {} ({:?})\nDescription: {}\nExisting question ids: {}\n\n\
         Original instructions:\n<<<\n{text}\n>>>\n\nThe user's notes about the project: {notes}",
        base.name,
        base.kind,
        base.description,
        if existing.is_empty() {
            "none".to_owned()
        } else {
            existing.join(", ")
        },
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Reply {
    questions: Vec<Question>,
    project_specifics: String,
}

/// The base template with the new questions and the section appended (original text
/// untouched); the error says what to fix.
pub fn parse_reply(base: &Template, text: &str) -> Result<Template, String> {
    let start = text.find('{').ok_or("the reply contains no JSON object")?;
    let end = text.rfind('}').ok_or("the reply contains no JSON object")?;
    let json = text
        .get(start..=end)
        .ok_or("the reply contains no JSON object")?;
    let reply: Reply =
        serde_json::from_str(json).map_err(|err| format!("the JSON does not match: {err}"))?;
    if reply.questions.is_empty() || reply.questions.len() > MAX_NEW_QUESTIONS {
        return Err(format!("give 1-{MAX_NEW_QUESTIONS} questions"));
    }
    let specifics = reply.project_specifics.trim();
    if specifics.is_empty() {
        return Err("projectSpecifics is empty".into());
    }
    let mut template = base.clone();
    for question in reply.questions {
        if template.questions.iter().any(|q| q.id == question.id) {
            return Err(format!("question id {} already exists", question.id));
        }
        template.questions.push(question);
    }
    if template.questions.len() > MAX_QUESTIONS {
        return Err(format!("at most {MAX_QUESTIONS} questions in total"));
    }
    template.instructions = format!(
        "{}\n\n{SECTION}\n\n{specifics}\n",
        base.instructions.trim_end()
    );
    let errors = validate(&template);
    if errors.is_empty() {
        Ok(template)
    } else {
        Err(errors.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::tests::sample;

    #[test]
    fn adds_questions_and_a_section_and_keeps_the_original_text() {
        let base = sample();
        let reply = r#"Sure: {"questions": [{"id": "testCmd", "label": "Test command",
            "type": "text", "default": "pnpm test", "detect": "testCommand"}],
            "projectSpecifics": "Run `{{testCmd}}` after each change."}"#;
        let template = parse_reply(&base, reply).expect("merged");
        assert!(template
            .instructions
            .starts_with(base.instructions.trim_end()));
        assert!(template
            .instructions
            .ends_with("## Project specifics\n\nRun `{{testCmd}}` after each change.\n"));
        assert_eq!(template.questions.len(), base.questions.len() + 1);
        assert_eq!(template.id, base.id);
    }

    #[test]
    fn rejects_unknown_placeholders_reused_ids_and_empty_replies() {
        let base = sample();
        let unknown = r#"{"questions": [{"id": "a", "label": "A", "type": "text"}],
            "projectSpecifics": "Use {{nope}}."}"#;
        assert!(parse_reply(&base, unknown).is_err());
        let reused = r#"{"questions": [{"id": "colors", "label": "A", "type": "text"}],
            "projectSpecifics": "x"}"#;
        assert!(parse_reply(&base, reused)
            .unwrap_err()
            .contains("already exists"));
        let empty = r#"{"questions": [], "projectSpecifics": "x"}"#;
        assert!(parse_reply(&base, empty).is_err());
        assert!(parse_reply(&base, "no json").is_err());
    }
}
