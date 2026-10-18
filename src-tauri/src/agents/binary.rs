//! Binary skill files (images, fonts) in plans: compared by hash, shown as a size in the
//! preview, written from base64.

use super::plan::{ConflictKind, PlannedFile, Status};
use super::registry::{hash_bytes, FileKind, InstalledFile};
use super::roots::Roots;
use super::targets::{Scope, Target};
use crate::error::{AppError, AppResult};
use crate::util::base64;

/// Binary files larger than this are refused (on disk or in a template).
const MAX_BINARY: u64 = 8 * 1024 * 1024;

pub fn describe(bytes: usize) -> String {
    format!("(binary file, {bytes} bytes)")
}

pub fn decode(data: &str, rel: &str) -> AppResult<Vec<u8>> {
    base64::decode(data).ok_or_else(|| AppError::InvalidInput(format!("{rel}: bad base64 data")))
}

pub fn planned(
    roots: &Roots,
    scope: Scope,
    target: Target,
    rel: &str,
    data: &str,
    recorded: Option<&InstalledFile>,
) -> AppResult<PlannedFile> {
    let after = decode(data, rel)?;
    let path = roots.resolve(scope, rel)?;
    let before = match std::fs::metadata(&path) {
        Ok(meta) if meta.is_file() && meta.len() <= MAX_BINARY => Some(std::fs::read(&path)?),
        Ok(_) => {
            return Err(AppError::InvalidInput(format!(
                "{rel} is not a file the installer can replace"
            )))
        }
        Err(_) => None,
    };
    let (status, conflict) = match &before {
        None => (Status::Create, None),
        Some(bytes) if *bytes == after => (Status::Unchanged, None),
        Some(bytes) => match recorded {
            Some(rec) if rec.hash == hash_bytes(bytes) => (Status::Update, None),
            Some(_) => (Status::Conflict, Some(ConflictKind::Edited)),
            None => (Status::Conflict, Some(ConflictKind::Foreign)),
        },
    };
    Ok(PlannedFile {
        target,
        scope,
        path: rel.to_owned(),
        display: Roots::display(scope, rel),
        kind: FileKind::File,
        status,
        conflict,
        before: before.map(|b| describe(b.len())),
        after: describe(after.len()),
        remove: false,
        created: false,
        data: Some(data.to_owned()),
    })
}

/// A whole file before its removal: what the preview shows (the text, or a size for binary
/// files) and its hash; `None` when it is gone already.
pub fn read_whole(roots: &Roots, scope: Scope, rel: &str) -> AppResult<Option<(String, String)>> {
    let path = roots.resolve(scope, rel)?;
    let Ok(meta) = std::fs::metadata(&path) else {
        return Ok(None);
    };
    if !meta.is_file() || meta.len() > MAX_BINARY {
        return Err(AppError::InvalidInput(format!(
            "{rel} is not a file the installer can remove"
        )));
    }
    let bytes = std::fs::read(&path)?;
    let digest = hash_bytes(&bytes);
    let shown = String::from_utf8(bytes).unwrap_or_else(|e| describe(e.into_bytes().len()));
    Ok(Some((shown, digest)))
}
