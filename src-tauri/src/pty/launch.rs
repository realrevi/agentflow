//! Turns a spawn request (kind + optional command/args) into a concrete program to run.

use std::path::PathBuf;

use super::cli::agent_binary;
use super::model::PtyKind;
use super::shell::default_shell;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<String>,
}

/// - `shell`: the default shell, interactive (command/args are ignored).
/// - `claude` / `codex` / `gemini`: the CLI found on PATH, with `args`.
/// - `custom`: `command` with `args`; with no args, `command` is a command line that runs
///   through the default shell (so pipes, quotes and env vars work as typed).
pub fn resolve_launch(kind: PtyKind, command: Option<&str>, args: &[String]) -> AppResult<Launch> {
    resolve_with(kind, command, args, |binary| which::which(binary).ok())
}

fn resolve_with(
    kind: PtyKind,
    command: Option<&str>,
    args: &[String],
    find: impl Fn(&str) -> Option<PathBuf>,
) -> AppResult<Launch> {
    match kind {
        PtyKind::Shell => {
            let shell = default_shell();
            let args = shell.interactive_args();
            Ok(Launch {
                program: shell.program,
                args,
            })
        }
        PtyKind::Claude | PtyKind::Codex | PtyKind::Gemini => {
            let binary = agent_binary(kind)
                .ok_or_else(|| AppError::Internal(format!("no binary for {kind:?}")))?;
            let program = find(binary)
                .ok_or_else(|| AppError::NotFound(format!("{binary} was not found in PATH")))?;
            Ok(Launch {
                program,
                args: args.to_vec(),
            })
        }
        PtyKind::Custom => {
            let command = command
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .ok_or_else(|| {
                    AppError::InvalidInput("a custom terminal needs a command".into())
                })?;
            if args.is_empty() {
                let shell = default_shell();
                let args = shell.command_args(command);
                Ok(Launch {
                    program: shell.program,
                    args,
                })
            } else {
                Ok(Launch {
                    program: PathBuf::from(command),
                    args: args.to_vec(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<PathBuf> {
        None
    }

    #[test]
    fn agent_kinds_resolve_on_path() {
        let found = |binary: &str| Some(PathBuf::from(format!("/opt/bin/{binary}")));
        let args = vec!["--help".to_string()];
        let launch = resolve_with(PtyKind::Gemini, None, &args, found).expect("found");
        assert_eq!(launch.program, PathBuf::from("/opt/bin/gemini"));
        assert_eq!(launch.args, args);

        match resolve_with(PtyKind::Claude, None, &[], none) {
            Err(AppError::NotFound(message)) => assert!(message.contains("claude")),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn custom_needs_a_command() {
        for command in [None, Some(""), Some("   ")] {
            assert!(matches!(
                resolve_with(PtyKind::Custom, command, &[], none),
                Err(AppError::InvalidInput(_))
            ));
        }
    }

    #[test]
    fn custom_without_args_runs_through_the_shell() {
        let launch = resolve_with(PtyKind::Custom, Some(" npm run dev "), &[], none).expect("ok");
        assert_eq!(launch.args.last().map(String::as_str), Some("npm run dev"));

        let args = vec!["-c".to_string(), "echo hi".to_string()];
        let direct = resolve_with(PtyKind::Custom, Some("sh"), &args, none).expect("ok");
        assert_eq!(direct.program, PathBuf::from("sh"));
        assert_eq!(direct.args, args);
    }
}
