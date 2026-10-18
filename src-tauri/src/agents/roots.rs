//! Where installer paths may land: the workspace (through the fs sandbox) or, for user
//! scope, only the folders each tool documents in the home directory.

use std::path::{Path, PathBuf};

use super::relpath::safe_relative;
use super::targets::Scope;
use crate::error::{AppError, AppResult};
use crate::fs::WorkspaceRoot;

/// Home-relative paths user-scope files may have (docs/agent-targets.md).
const USER_FOLDERS: [&str; 4] = [
    ".claude/agents/",
    ".codex/agents/",
    ".gemini/agents/",
    ".gemini/commands/",
];
const USER_FILES: [&str; 3] = [".claude/CLAUDE.md", ".codex/AGENTS.md", ".gemini/GEMINI.md"];
/// Skill folders: `<folder><skill id>/<safe relative path>`.
const USER_SKILLS: [&str; 3] = [".claude/skills/", ".agents/skills/", ".gemini/skills/"];

#[derive(Debug, Clone)]
pub struct Roots {
    pub workspace: WorkspaceRoot,
    pub home: Option<PathBuf>,
}

impl Roots {
    pub fn new(workspace: &Path, home: Option<PathBuf>) -> AppResult<Self> {
        Ok(Self {
            workspace: WorkspaceRoot::open(workspace)?,
            home,
        })
    }

    /// The absolute path of `rel` in `scope`; user paths outside the documented folders and
    /// anything escaping its root (`..`, symlinks) are refused.
    pub fn resolve(&self, scope: Scope, rel: &str) -> AppResult<PathBuf> {
        match scope {
            Scope::Project => self.workspace.resolve_new(rel),
            Scope::User => {
                let documented = USER_FILES.contains(&rel)
                    || USER_FOLDERS.iter().any(|folder| {
                        rel.strip_prefix(folder)
                            .is_some_and(|name| !name.is_empty() && !name.contains('/'))
                    })
                    || USER_SKILLS.iter().any(|folder| {
                        rel.strip_prefix(folder).is_some_and(|rest| {
                            rest.contains('/') && safe_relative(rest).is_ok_and(|p| p == rest)
                        })
                    });
                if !documented {
                    return Err(AppError::InvalidInput(format!(
                        "{rel} is not a documented user folder"
                    )));
                }
                let home = self.home.as_deref().ok_or_else(|| {
                    AppError::NotFound("the home folder could not be found".into())
                })?;
                WorkspaceRoot::open(home)?.resolve_new(rel)
            }
        }
    }

    /// A folder inside the workspace or home (for removing empty skill folders).
    pub fn resolve_dir(&self, scope: Scope, rel: &str) -> AppResult<PathBuf> {
        match scope {
            Scope::Project => self.workspace.resolve_new(rel),
            Scope::User => {
                let home = self.home.as_deref().ok_or_else(|| {
                    AppError::NotFound("the home folder could not be found".into())
                })?;
                WorkspaceRoot::open(home)?.resolve_new(rel)
            }
        }
    }

    /// How a path is shown: workspace-relative, or `~/...` for user scope.
    pub fn display(scope: Scope, rel: &str) -> String {
        match scope {
            Scope::Project => rel.to_owned(),
            Scope::User => format!("~/{rel}"),
        }
    }
}
