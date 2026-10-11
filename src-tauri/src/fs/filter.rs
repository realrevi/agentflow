//! Decides which watcher events the frontend sees: the same rules as the listing
//! (ignore files, hidden files, never `.git`), checked per path with `IncrementalIgnore`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use ignore::IncrementalIgnore;

use super::paths::relative_to;
use super::walk::{walker, GIT_DIR};

pub struct EventFilter {
    root: PathBuf,
    show_hidden: bool,
    matcher: Option<IncrementalIgnore>,
}

impl EventFilter {
    pub fn new(root: PathBuf, show_hidden: bool) -> Self {
        let matcher = build(&root, show_hidden);
        Self {
            root,
            show_hidden,
            matcher,
        }
    }

    /// Reloads the ignore files (call when a `.gitignore` or `.ignore` changed).
    pub fn reload(&mut self) {
        self.matcher = build(&self.root, self.show_hidden);
    }

    /// The relative path when `path` is visible in the tree, otherwise `None`.
    pub fn visible(&mut self, path: &Path) -> Option<String> {
        let rel = path.strip_prefix(&self.root).ok()?;
        if rel
            .components()
            .any(|part| part.as_os_str() == OsStr::new(GIT_DIR))
        {
            return None;
        }
        if let Some(matcher) = &mut self.matcher {
            if matcher.matched(rel, path.is_dir()).is_ignore() {
                return None;
            }
        }
        relative_to(&self.root, path)
    }
}

fn build(root: &Path, show_hidden: bool) -> Option<IncrementalIgnore> {
    walker(root, show_hidden)
        .build_matchers()
        .into_iter()
        .next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_dir::TestDir;
    use std::fs;

    #[test]
    fn hides_ignored_hidden_and_git_paths() {
        let dir = TestDir::new("fs-filter");
        let root = dir.path().canonicalize().expect("canonical");
        fs::write(root.join(".gitignore"), "target/\n*.log\n").expect("write");
        fs::create_dir_all(root.join("target/debug")).expect("mkdir");
        fs::create_dir_all(root.join("src")).expect("mkdir");

        let mut filter = EventFilter::new(root.clone(), false);
        assert_eq!(
            filter.visible(&root.join("src/main.rs")).as_deref(),
            Some("src/main.rs")
        );
        assert_eq!(filter.visible(&root.join("target/debug/app")), None);
        assert_eq!(filter.visible(&root.join("build.log")), None);
        assert_eq!(filter.visible(&root.join(".git/index")), None);
        assert_eq!(filter.visible(&root.join(".env")), None);
        assert_eq!(filter.visible(Path::new("/elsewhere/file")), None);

        let mut with_hidden = EventFilter::new(root.clone(), true);
        assert_eq!(
            with_hidden.visible(&root.join(".env")).as_deref(),
            Some(".env")
        );
        assert_eq!(
            with_hidden.visible(&root.join(".git/index")),
            None,
            ".git stays hidden"
        );

        fs::write(root.join(".gitignore"), "").expect("write");
        filter.reload();
        assert!(filter.visible(&root.join("build.log")).is_some());
    }
}
