//! Changes to the workspace: atomic saves with an mtime check, create, rename, trash.
//! Paths are already resolved and checked by `WorkspaceRoot`.

use std::fs::{self, OpenOptions};
use std::io;
use std::path::Path;

use super::model::{EntryKind, FileStat};
use super::read::mtime_ms;
use crate::error::{AppError, AppResult};
use crate::util::write_atomic_with;

/// Writes `content` atomically. With `expected_mtime`, the file must still have that
/// modification time (a missing file is a conflict too); `None` writes unconditionally.
pub fn write_file(path: &Path, content: &[u8], expected_mtime: Option<u64>) -> AppResult<FileStat> {
    let current = match fs::metadata(path) {
        Ok(meta) => Some(meta),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => return Err(err.into()),
    };
    if let Some(expected) = expected_mtime {
        match &current {
            None => return Err(AppError::Conflict("the file was deleted on disk".into())),
            Some(meta) if mtime_ms(meta) != expected => {
                return Err(AppError::Conflict("the file changed on disk".into()));
            }
            Some(_) => {}
        }
    }
    if current.as_ref().is_some_and(|meta| !meta.is_file()) {
        return Err(AppError::InvalidInput(format!(
            "not a file: {}",
            path.display()
        )));
    }
    write_atomic_with(path, content, current.map(|meta| meta.permissions()))?;
    let meta = fs::metadata(path)?;
    Ok(FileStat {
        size: meta.len(),
        mtime: mtime_ms(&meta),
    })
}

/// Creates an empty file or a folder (and any missing parent folders). Never overwrites.
pub fn create(path: &Path, kind: EntryKind) -> AppResult<()> {
    if fs::symlink_metadata(path).is_ok() {
        return Err(already_exists(path));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    match kind {
        EntryKind::File => {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|err| exists_or_io(err, path))?;
        }
        EntryKind::Dir => fs::create_dir(path).map_err(|err| exists_or_io(err, path))?,
    }
    Ok(())
}

/// Renames or moves an entry. Never overwrites, except a case-only rename of itself.
pub fn rename(from: &Path, to: &Path) -> AppResult<()> {
    if to.starts_with(from) && to != from {
        return Err(AppError::InvalidInput(
            "a folder cannot be moved into itself".into(),
        ));
    }
    if fs::symlink_metadata(to).is_ok() && !is_case_change(from, to) {
        return Err(already_exists(to));
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(from, to)?;
    Ok(())
}

/// Moves an entry to the OS trash (recycle bin). Never deletes permanently.
pub fn move_to_trash(path: &Path) -> AppResult<()> {
    trash::delete(path).map_err(|err| {
        AppError::Io(io::Error::other(format!(
            "could not move {} to the trash: {err}",
            path.display()
        )))
    })
}

/// `Readme.md` -> `README.md` on a case-insensitive file system names the same file.
fn is_case_change(from: &Path, to: &Path) -> bool {
    let names = (from.file_name(), to.file_name());
    let (Some(a), Some(b)) = names else {
        return false;
    };
    from.parent() == to.parent()
        && a != b
        && a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

fn already_exists(path: &Path) -> AppError {
    let name = path.file_name().unwrap_or(path.as_os_str());
    AppError::InvalidInput(format!("already exists: {}", name.to_string_lossy()))
}

fn exists_or_io(err: io::Error, path: &Path) -> AppError {
    if err.kind() == io::ErrorKind::AlreadyExists {
        already_exists(path)
    } else {
        AppError::Io(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_dir::TestDir;

    #[test]
    fn writes_atomically_and_checks_the_mtime() {
        let dir = TestDir::new("fs-write");
        let path = dir.path().join("notes.txt");
        let first = write_file(&path, b"one", None).expect("create");
        assert_eq!(fs::read_to_string(&path).expect("read"), "one");
        assert_eq!(first.size, 3);

        let second = write_file(&path, b"two", Some(first.mtime)).expect("save");
        assert_eq!(fs::read_to_string(&path).expect("read"), "two");

        let stale = second.mtime.saturating_sub(10_000);
        assert!(matches!(
            write_file(&path, b"three", Some(stale)),
            Err(AppError::Conflict(_))
        ));
        assert_eq!(fs::read_to_string(&path).expect("read"), "two", "untouched");

        let leftovers = fs::read_dir(dir.path()).expect("list").count();
        assert_eq!(leftovers, 1, "no temp files left behind");
    }

    #[test]
    fn a_deleted_file_is_a_conflict() {
        let dir = TestDir::new("fs-write");
        let path = dir.path().join("gone.txt");
        assert!(matches!(
            write_file(&path, b"x", Some(1)),
            Err(AppError::Conflict(_))
        ));
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn saving_keeps_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TestDir::new("fs-write");
        let path = dir.path().join("run.sh");
        fs::write(&path, "#!/bin/sh\n").expect("write");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
        write_file(&path, b"#!/bin/sh\necho hi\n", None).expect("save");
        let mode = fs::metadata(&path).expect("meta").permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
    }

    #[test]
    fn create_and_rename_never_overwrite() {
        let dir = TestDir::new("fs-write");
        let file = dir.path().join("a/b/new.txt");
        create(&file, EntryKind::File).expect("create file");
        assert!(file.is_file());
        assert!(create(&file, EntryKind::File).is_err());
        create(&dir.path().join("folder"), EntryKind::Dir).expect("create dir");

        let target = dir.path().join("folder/moved.txt");
        rename(&file, &target).expect("move");
        assert!(target.is_file() && !file.exists());

        fs::write(dir.path().join("other.txt"), "x").expect("write");
        assert!(rename(&dir.path().join("other.txt"), &target).is_err());
        assert!(rename(&dir.path().join("folder"), &dir.path().join("folder/sub")).is_err());
    }

    #[test]
    fn delete_moves_to_the_trash() {
        let dir = TestDir::new("fs-trash");
        let path = dir.path().join("trash-me.txt");
        fs::write(&path, "bye").expect("write");
        if let Err(err) = move_to_trash(&path) {
            // Containers often lack a desktop trash; the call must still never delete.
            eprintln!("note: skipping trash test, no trash available here: {err}");
            assert!(path.exists(), "a failed trash must not delete the file");
            return;
        }
        assert!(!path.exists());
        #[cfg(any(target_os = "linux", windows))]
        {
            let parent = dir.path().canonicalize().expect("canonical");
            let items = trash::os_limited::list().expect("list trash");
            let ours: Vec<_> = items
                .into_iter()
                .filter(|item| item.original_parent == parent)
                .collect();
            assert_eq!(ours.len(), 1, "the file is in the trash");
            trash::os_limited::purge_all(ours).expect("clean up the trash");
        }
    }
}
