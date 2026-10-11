//! Resolves frontend paths (relative, `/`-separated) inside the workspace root.
//! Everything is canonicalized and checked against the canonical root, so `..`,
//! absolute paths and symlinks pointing outside the workspace are all rejected.

use std::io;
use std::path::{Component, Path, PathBuf};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct WorkspaceRoot {
    root: PathBuf,
}

impl WorkspaceRoot {
    pub fn open(path: &Path) -> AppResult<Self> {
        let root = std::fs::canonicalize(path).map_err(|err| io_error(err, path.display()))?;
        if !root.is_dir() {
            return Err(AppError::InvalidInput(format!(
                "not a folder: {}",
                path.display()
            )));
        }
        Ok(Self { root })
    }

    /// The canonical root.
    pub fn path(&self) -> &Path {
        &self.root
    }

    /// An existing file or folder, with symlinks followed (they must stay inside).
    pub fn resolve(&self, rel: &str) -> AppResult<PathBuf> {
        let joined = self.join(&segments(rel)?);
        let canonical = std::fs::canonicalize(&joined).map_err(|err| io_error(err, rel))?;
        self.check_inside(canonical, rel)
    }

    /// An existing entry itself, not followed if it is a symlink (for rename and delete).
    pub fn resolve_entry(&self, rel: &str) -> AppResult<PathBuf> {
        let parts = segments(rel)?;
        let Some((name, parent)) = parts.split_last() else {
            return Err(AppError::InvalidInput(
                "the workspace folder itself cannot be changed".into(),
            ));
        };
        let parent = self.resolve(&parent.join("/"))?;
        let path = parent.join(name);
        std::fs::symlink_metadata(&path).map_err(|err| io_error(err, rel))?;
        Ok(path)
    }

    /// A path that may not exist yet (create, rename target, new file). The nearest
    /// existing ancestor must resolve inside the workspace.
    pub fn resolve_new(&self, rel: &str) -> AppResult<PathBuf> {
        let parts = segments(rel)?;
        if parts.is_empty() {
            return Err(AppError::InvalidInput("a name is required".into()));
        }
        let mut existing = parts.len() - 1;
        while existing > 0 && std::fs::symlink_metadata(self.join(&parts[..existing])).is_err() {
            existing -= 1;
        }
        let base = self.resolve(&parts[..existing].join("/"))?;
        Ok(parts[existing..]
            .iter()
            .fold(base, |path, part| path.join(part)))
    }

    /// The relative `/`-separated form of an absolute path inside the root. `None` for
    /// paths outside the root or with names that are not valid UTF-8.
    pub fn relative(&self, path: &Path) -> Option<String> {
        relative_to(&self.root, path)
    }

    fn join(&self, parts: &[&str]) -> PathBuf {
        parts
            .iter()
            .fold(self.root.clone(), |path, part| path.join(part))
    }

    fn check_inside(&self, path: PathBuf, rel: &str) -> AppResult<PathBuf> {
        if path.starts_with(&self.root) {
            Ok(path)
        } else {
            Err(outside(rel))
        }
    }
}

pub fn relative_to(root: &Path, path: &Path) -> Option<String> {
    let rest = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in rest.components() {
        match component {
            Component::Normal(name) => parts.push(name.to_str()?),
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(parts.join("/"))
}

/// Splits a relative path into plain names. `.` and empty segments are skipped;
/// absolute paths, drive prefixes and `..` are rejected.
fn segments(rel: &str) -> AppResult<Vec<&str>> {
    let mut parts = Vec::new();
    for part in rel.split(is_separator) {
        if part.is_empty() || part == "." {
            continue;
        }
        let mut components = Path::new(part).components();
        let single = matches!(
            (components.next(), components.next()),
            (Some(Component::Normal(_)), None)
        );
        if !single || part.contains('\0') {
            return Err(outside(rel));
        }
        parts.push(part);
    }
    if Path::new(rel).has_root() {
        return Err(outside(rel));
    }
    Ok(parts)
}

fn is_separator(c: char) -> bool {
    c == '/' || (cfg!(windows) && c == '\\')
}

fn outside(rel: &str) -> AppError {
    AppError::InvalidInput(format!("path is outside the workspace: {rel}"))
}

fn io_error(err: io::Error, path: impl std::fmt::Display) -> AppError {
    if err.kind() == io::ErrorKind::NotFound {
        AppError::NotFound(path.to_string())
    } else {
        AppError::Io(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_dir::TestDir;
    use std::fs;

    fn setup() -> (TestDir, WorkspaceRoot) {
        let dir = TestDir::new("fs-paths");
        fs::create_dir_all(dir.path().join("ws/src")).expect("mkdir");
        fs::write(dir.path().join("ws/src/main.rs"), "fn main() {}").expect("write");
        fs::write(dir.path().join("secret.txt"), "nope").expect("write");
        let root = WorkspaceRoot::open(&dir.path().join("ws")).expect("open");
        (dir, root)
    }

    #[test]
    fn resolves_relative_paths_inside() {
        let (_dir, root) = setup();
        let file = root.resolve("src/main.rs").expect("resolve");
        assert!(file.ends_with("src/main.rs"));
        assert_eq!(root.resolve("").expect("root"), root.path());
        assert_eq!(
            root.resolve("./src/").expect("dir"),
            root.path().join("src")
        );
        assert_eq!(root.relative(&file).as_deref(), Some("src/main.rs"));
    }

    #[test]
    fn rejects_traversal_and_absolute_paths() {
        let (dir, root) = setup();
        for bad in ["../secret.txt", "src/../../secret.txt", "src/.."] {
            assert!(
                matches!(root.resolve(bad), Err(AppError::InvalidInput(_))),
                "{bad} must be rejected"
            );
            assert!(root.resolve_new(bad).is_err(), "{bad} must be rejected");
        }
        let absolute = dir.path().join("secret.txt");
        let absolute = absolute.to_string_lossy();
        assert!(matches!(
            root.resolve(&absolute),
            Err(AppError::InvalidInput(_))
        ));
        assert!(
            root.resolve_entry("").is_err(),
            "the root cannot be deleted"
        );
    }

    #[test]
    fn missing_paths_are_not_found() {
        let (_dir, root) = setup();
        assert!(matches!(
            root.resolve("nope.txt"),
            Err(AppError::NotFound(_))
        ));
        let created = root.resolve_new("src/new/deep.txt").expect("new");
        assert_eq!(created, root.path().join("src/new/deep.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_escaping_the_root_are_rejected() {
        use std::os::unix::fs::symlink;
        let (dir, root) = setup();
        symlink(dir.path().join("secret.txt"), root.path().join("leak.txt")).expect("link");
        symlink(dir.path(), root.path().join("outside")).expect("link");
        symlink(root.path().join("src"), root.path().join("inner")).expect("link");

        assert!(matches!(
            root.resolve("leak.txt"),
            Err(AppError::InvalidInput(_))
        ));
        assert!(matches!(
            root.resolve("outside/secret.txt"),
            Err(AppError::InvalidInput(_))
        ));
        assert!(root.resolve_new("outside/new.txt").is_err());
        assert!(
            root.resolve("inner/main.rs").is_ok(),
            "links inside are fine"
        );
        // The link itself can still be renamed or deleted; its target is untouched.
        assert_eq!(
            root.resolve_entry("leak.txt").expect("entry"),
            root.path().join("leak.txt")
        );
    }
}
