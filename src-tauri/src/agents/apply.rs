//! Writing: install or update (a re-planned preview, confirmed conflicts only) and
//! uninstall (our files to the trash, our sections taken out; edited ones only when the user
//! confirms them). The registry records every file with its hash.

use std::path::Path;

use super::plan::{check, plan, removal, Confirmations, InstallRequest, Plan, PlannedFile, Status};
use super::registry::{hash, FileKind, Installed, InstalledFile, Registry};
use super::roots::Roots;
use super::section;
use crate::error::{AppError, AppResult};
use crate::util::{now_ms, write_atomic};

/// Writes the plan (re-planned here, so a stale preview cannot be applied) and records it.
pub fn install(
    roots: &Roots,
    request: &InstallRequest,
    confirm: &Confirmations,
    trash: impl Fn(&Path) -> AppResult<()>,
) -> AppResult<(Plan, Installed)> {
    let root = roots.workspace.path().to_path_buf();
    let mut registry = Registry::load(&root);
    let plan = plan(roots, &registry, request)?;
    check(&plan, confirm)?;
    let previous = registry.get(&request.template.id).cloned();
    let mut files = Vec::new();
    for file in &plan.files {
        let path = roots.resolve(file.scope, &file.path)?;
        if file.remove {
            if file.before.is_some() {
                apply_removal(file, &path, &trash)?;
            }
            continue;
        }
        if file.status != Status::Unchanged {
            write_atomic(&path, file.after.as_bytes())?;
        }
        let created_before = previous
            .as_ref()
            .and_then(|p| {
                p.files
                    .iter()
                    .find(|f| f.scope == file.scope && f.path == file.path)
            })
            .is_some_and(|f| f.created);
        files.push(InstalledFile {
            target: file.target,
            scope: file.scope,
            path: file.path.clone(),
            kind: file.kind,
            hash: recorded_hash(file, &request.template.id)?,
            created: created_before || file.before.is_none(),
        });
    }
    let now = now_ms();
    let entry = Installed {
        id: request.template.id.clone(),
        template: request.template.clone(),
        answers: request.answers.clone(),
        targets: request.targets.clone(),
        scope: request.scope,
        installed_at: previous.map_or(now, |p| p.installed_at),
        updated_at: now,
        files,
    };
    registry.put(entry.clone());
    registry.save(&root)?;
    Ok((plan, entry))
}

fn recorded_hash(file: &PlannedFile, id: &str) -> AppResult<String> {
    Ok(match file.kind {
        FileKind::File => hash(&file.after),
        FileKind::Section => hash(&section::content(&file.after, id)?.unwrap_or_default()),
    })
}

/// What uninstalling `id` would do, file by file (edited files are conflicts).
pub fn uninstall_plan(roots: &Roots, id: &str) -> AppResult<Plan> {
    let registry = Registry::load(roots.workspace.path());
    let installed = registry
        .get(id)
        .ok_or_else(|| AppError::NotFound(format!("installed agent {id}")))?;
    let files = installed
        .files
        .iter()
        .map(|file| removal(roots, file, id))
        .collect::<AppResult<Vec<_>>>()?;
    Ok(Plan {
        user_scope: files.iter().any(|f| f.scope == super::targets::Scope::User),
        files,
    })
}

/// Removes our files and sections; edited ones need a confirmation (or stay, if listed in
/// `keep`). The registry entry goes last.
pub fn uninstall(
    roots: &Roots,
    id: &str,
    confirm: &Confirmations,
    keep: &[String],
    trash: impl Fn(&Path) -> AppResult<()>,
) -> AppResult<Plan> {
    let plan = uninstall_plan(roots, id)?;
    let kept = |f: &PlannedFile| keep.contains(&f.display);
    let pending = Plan {
        user_scope: plan.user_scope,
        files: plan.files.iter().filter(|f| !kept(f)).cloned().collect(),
    };
    check(&pending, confirm)?;
    for file in &pending.files {
        if file.before.is_some() {
            let path = roots.resolve(file.scope, &file.path)?;
            apply_removal(file, &path, &trash)?;
        }
    }
    let root = roots.workspace.path();
    let mut registry = Registry::load(root);
    registry.remove(id);
    registry.save(root)?;
    Ok(plan)
}

/// A whole file goes to the trash; a section is taken out, and a shared file the installer
/// created goes to the trash once nothing else is left in it.
pub(super) fn apply_removal(
    file: &PlannedFile,
    path: &Path,
    trash: &impl Fn(&Path) -> AppResult<()>,
) -> AppResult<()> {
    match file.kind {
        FileKind::File => trash(path),
        FileKind::Section if file.created && file.after.trim().is_empty() => trash(path),
        FileKind::Section => Ok(write_atomic(path, file.after.as_bytes())?),
    }
}
