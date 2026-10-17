//! Install and update: render the template for each chosen target, compare with what is on
//! disk and in the registry, and show every file before anything is written. A file the
//! installer did not create, or one the user edited since (hash mismatch), is a conflict that
//! needs an explicit confirmation; user-scope writes need their own confirmation.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::placeholders::{render, values};
use super::registry::{hash, FileKind, InstalledFile, Registry};
use super::roots::Roots;
use super::section;
use super::targets::{self, Content, Rendered, Scope, Target};
use super::template::{Answers, Template};
use super::validate::validate;
use crate::error::{AppError, AppResult};

/// Shared files larger than this are not edited.
const MAX_EXISTING: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
    pub template: Template,
    pub answers: Answers,
    pub targets: Vec<Target>,
    #[serde(default)]
    pub scope: Scope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Create,
    Update,
    Unchanged,
    Conflict,
    /// A file of an earlier install this one no longer produces.
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictKind {
    /// The file exists and the installer did not create it.
    Foreign,
    /// The installer wrote it, then it was edited.
    Edited,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedFile {
    pub target: Target,
    pub scope: Scope,
    /// Relative to the workspace or home folder.
    pub path: String,
    /// `CLAUDE.md` or `~/.claude/agents/x.md`.
    pub display: String,
    pub kind: FileKind,
    pub status: Status,
    pub conflict: Option<ConflictKind>,
    pub before: Option<String>,
    pub after: String,
    /// A removal (file to the trash, or our section taken out): `after` is what remains.
    pub remove: bool,
    /// The installer created this shared file (it may go once our section is out).
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub files: Vec<PlannedFile>,
    pub user_scope: bool,
}

fn read_existing(roots: &Roots, scope: Scope, rel: &str) -> AppResult<Option<String>> {
    let path = roots.resolve(scope, rel)?;
    let Ok(meta) = std::fs::metadata(&path) else {
        return Ok(None);
    };
    if !meta.is_file() || meta.len() > MAX_EXISTING {
        return Err(AppError::InvalidInput(format!(
            "{rel} is not a file the installer can edit"
        )));
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|_| AppError::InvalidInput(format!("{rel} is not a UTF-8 text file")))?;
    Ok(Some(text))
}

/// The instructions with placeholders filled, after validating the template.
pub fn rendered_instructions(
    template: &Template,
    answers: &Answers,
    workspace_name: &str,
) -> AppResult<String> {
    let errors = validate(template);
    if !errors.is_empty() {
        return Err(AppError::InvalidInput(errors.join("; ")));
    }
    let values = values(template, answers, workspace_name)?;
    render(&template.instructions, &values)
}

pub fn plan(roots: &Roots, registry: &Registry, request: &InstallRequest) -> AppResult<Plan> {
    let template = &request.template;
    if request.targets.is_empty() {
        return Err(AppError::InvalidInput("pick at least one target".into()));
    }
    let workspace_name = roots
        .workspace
        .path()
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let instructions = rendered_instructions(template, &request.answers, workspace_name)?;
    let rendered = Rendered {
        template,
        instructions,
    };
    let previous = registry.get(&template.id);
    let mut files = Vec::new();
    let mut produced = BTreeSet::new();
    for &target in &request.targets {
        if request.scope == Scope::User && !target.has_user_scope() {
            return Err(AppError::InvalidInput(format!(
                "{target:?} has no documented user folder; install it in the project"
            )));
        }
        for output in targets::render(&rendered, target, request.scope) {
            let scope = request.scope;
            let before = read_existing(roots, scope, &output.path)?;
            let (kind, after) = match &output.content {
                Content::File { text } => (FileKind::File, text.clone()),
                Content::Section { body } => (
                    FileKind::Section,
                    section::upsert(before.as_deref().unwrap_or(""), &template.id, body)?,
                ),
            };
            let recorded = previous.and_then(|p| {
                p.files
                    .iter()
                    .find(|f| f.scope == scope && f.path == output.path && f.kind == kind)
            });
            let (status, conflict) =
                status(kind, before.as_deref(), &after, recorded, &template.id)?;
            produced.insert((scope, output.path.clone(), kind));
            files.push(PlannedFile {
                target,
                scope,
                display: Roots::display(scope, &output.path),
                path: output.path,
                kind,
                status,
                conflict,
                before,
                after,
                remove: false,
                created: false,
            });
        }
    }
    for old in previous.map(|p| p.files.as_slice()).unwrap_or_default() {
        if produced.contains(&(old.scope, old.path.clone(), old.kind)) {
            continue;
        }
        files.push(removal(roots, old, &template.id)?);
    }
    Ok(Plan {
        user_scope: files.iter().any(|f| f.scope == Scope::User),
        files,
    })
}

fn status(
    kind: FileKind,
    before: Option<&str>,
    after: &str,
    recorded: Option<&InstalledFile>,
    id: &str,
) -> AppResult<(Status, Option<ConflictKind>)> {
    let Some(before) = before else {
        return Ok((Status::Create, None));
    };
    if before == after {
        return Ok((Status::Unchanged, None));
    }
    let current = match kind {
        FileKind::File => Some(hash(before)),
        FileKind::Section => section::content(before, id)?.map(|c| hash(&c)),
    };
    Ok(match (kind, current, recorded) {
        // Our own file or section, untouched since: safe to update.
        (_, Some(now), Some(rec)) if now == rec.hash => (Status::Update, None),
        // A shared file without our section yet: we only add the section.
        (FileKind::Section, None, _) => (Status::Update, None),
        (_, _, Some(_)) => (Status::Conflict, Some(ConflictKind::Edited)),
        (FileKind::File, _, None) => (Status::Conflict, Some(ConflictKind::Foreign)),
        (FileKind::Section, Some(_), None) => (Status::Conflict, Some(ConflictKind::Edited)),
    })
}

/// A file of the previous install that the new one does not write.
pub(super) fn removal(roots: &Roots, old: &InstalledFile, id: &str) -> AppResult<PlannedFile> {
    let before = read_existing(roots, old.scope, &old.path)?;
    let (after, edited) = match (&before, old.kind) {
        (None, _) => (String::new(), false),
        (Some(text), FileKind::File) => (String::new(), hash(text) != old.hash),
        (Some(text), FileKind::Section) => {
            let current = section::content(text, id)?;
            (
                section::remove(text, id)?,
                current.is_some_and(|c| hash(&c) != old.hash),
            )
        }
    };
    Ok(PlannedFile {
        target: old.target,
        scope: old.scope,
        path: old.path.clone(),
        display: Roots::display(old.scope, &old.path),
        kind: old.kind,
        status: if before.is_none() {
            Status::Unchanged
        } else if edited {
            Status::Conflict
        } else {
            Status::Remove
        },
        conflict: edited.then_some(ConflictKind::Edited),
        before,
        after,
        remove: true,
        created: old.created,
    })
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Confirmations {
    /// Conflicting paths (as `display`) the user agreed to overwrite or remove.
    #[serde(default)]
    pub overwrite: Vec<String>,
    /// The separate "write to my home folder" confirmation.
    #[serde(default)]
    pub user_scope: bool,
}

pub(super) fn check(plan: &Plan, confirm: &Confirmations) -> AppResult<()> {
    if plan.user_scope && !confirm.user_scope {
        return Err(AppError::Conflict(
            "writing to the home folder needs its own confirmation".into(),
        ));
    }
    let open: Vec<&str> = plan
        .files
        .iter()
        .filter(|f| f.conflict.is_some() && !confirm.overwrite.contains(&f.display))
        .map(|f| f.display.as_str())
        .collect();
    if !open.is_empty() {
        return Err(AppError::Conflict(format!(
            "these files changed outside the installer; confirm them in the preview: {}",
            open.join(", ")
        )));
    }
    Ok(())
}
