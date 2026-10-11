//! Crash-safe file writes: write a sibling temp file, fsync, then rename over the target.

use std::ffi::OsString;
use std::fs::{self, File, Permissions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Atomically replaces `path` with `bytes`, creating parent directories as needed.
/// Readers never observe a half-written file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_atomic_with(path, bytes, None)
}

/// Like `write_atomic`, giving the new file `permissions` (e.g. the replaced file's, so an
/// executable script stays executable).
pub fn write_atomic_with(
    path: &Path,
    bytes: &[u8],
    permissions: Option<Permissions>,
) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    fs::create_dir_all(dir)?;
    let tmp = temp_path(path)?;

    let written = write_and_sync(&tmp, bytes)
        .and_then(|()| match permissions {
            Some(permissions) => fs::set_permissions(&tmp, permissions),
            None => Ok(()),
        })
        .and_then(|()| fs::rename(&tmp, path));
    if written.is_err() {
        // Best effort cleanup; the original error is what matters.
        let _ = fs::remove_file(&tmp);
    }
    written?;
    sync_dir(dir);
    Ok(())
}

fn temp_path(path: &Path) -> io::Result<PathBuf> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let mut tmp_name = OsString::from(".");
    tmp_name.push(name);
    tmp_name.push(".tmp");
    Ok(path.with_file_name(tmp_name))
}

fn write_and_sync(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Persists the rename itself on Unix. Not supported (or needed) on Windows.
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(handle) = File::open(dir) {
        let _ = handle.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_dir::TestDir;

    #[test]
    fn writes_and_replaces_without_leaving_temp_files() {
        let dir = TestDir::new("atomic");
        let target = dir.path().join("nested").join("file.json");

        write_atomic(&target, b"first").expect("first write");
        write_atomic(&target, b"second").expect("second write");

        assert_eq!(fs::read_to_string(&target).expect("read"), "second");
        let leftovers: Vec<_> = fs::read_dir(target.parent().expect("parent"))
            .expect("list")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
    }
}
