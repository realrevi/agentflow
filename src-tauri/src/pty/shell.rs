//! Default shell per OS and how to run a command line through it.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// A program plus the arguments it needs to start interactively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shell {
    pub program: PathBuf,
    flavor: Flavor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flavor {
    Posix,
    PowerShell,
    Cmd,
}

impl Shell {
    /// Arguments for an interactive session. macOS terminals start login shells so the
    /// user's profile (and PATH) is loaded even when the app was launched from Finder.
    pub fn interactive_args(&self) -> Vec<String> {
        match self.flavor {
            Flavor::Posix if cfg!(target_os = "macos") => vec!["-l".into()],
            Flavor::Posix | Flavor::Cmd => Vec::new(),
            Flavor::PowerShell => vec!["-NoLogo".into()],
        }
    }

    /// Arguments that run `line` through this shell and exit when it finishes.
    pub fn command_args(&self, line: &str) -> Vec<String> {
        let mut args = match self.flavor {
            Flavor::Posix if cfg!(target_os = "macos") => vec!["-l".into(), "-c".into()],
            Flavor::Posix => vec!["-c".into()],
            Flavor::PowerShell => vec!["-NoLogo".into(), "-Command".into()],
            Flavor::Cmd => vec!["/C".into()],
        };
        args.push(line.into());
        args
    }
}

/// The user's shell: `$SHELL` on macOS/Linux, pwsh / powershell / cmd on Windows.
pub fn default_shell() -> Shell {
    #[cfg(windows)]
    {
        resolve_windows_shell(|name| which::which(name).ok())
    }
    #[cfg(not(windows))]
    {
        resolve_unix_shell(std::env::var_os("SHELL"), Path::is_file)
    }
}

/// `$SHELL` if it points at an existing file, else `/bin/bash`, else `/bin/sh`.
#[cfg_attr(windows, allow(dead_code))]
pub fn resolve_unix_shell(env_shell: Option<OsString>, exists: impl Fn(&Path) -> bool) -> Shell {
    let from_env = env_shell
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && exists(path));
    let program = from_env
        .or_else(|| {
            ["/bin/bash", "/bin/sh"]
                .into_iter()
                .map(PathBuf::from)
                .find(|path| exists(path))
        })
        .unwrap_or_else(|| PathBuf::from("/bin/sh"));
    Shell {
        program,
        flavor: Flavor::Posix,
    }
}

/// The first of pwsh, powershell and cmd found on PATH (cmd.exe as a last resort).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn resolve_windows_shell(find: impl Fn(&str) -> Option<PathBuf>) -> Shell {
    let candidates = [
        ("pwsh", Flavor::PowerShell),
        ("powershell", Flavor::PowerShell),
        ("cmd", Flavor::Cmd),
    ];
    candidates
        .into_iter()
        .find_map(|(name, flavor)| find(name).map(|program| Shell { program, flavor }))
        .unwrap_or_else(|| Shell {
            program: PathBuf::from("cmd.exe"),
            flavor: Flavor::Cmd,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn only(paths: &'static [&'static str]) -> impl Fn(&Path) -> bool {
        move |path| paths.iter().any(|p| Path::new(p) == path)
    }

    #[test]
    fn unix_prefers_existing_shell_env() {
        let shell = resolve_unix_shell(Some("/usr/bin/zsh".into()), only(&["/usr/bin/zsh"]));
        assert_eq!(shell.program, PathBuf::from("/usr/bin/zsh"));
    }

    #[test]
    fn unix_falls_back_to_bash_then_sh() {
        let missing = Some(OsString::from("/nope/fish"));
        let bash = resolve_unix_shell(missing.clone(), only(&["/bin/bash", "/bin/sh"]));
        assert_eq!(bash.program, PathBuf::from("/bin/bash"));
        let sh = resolve_unix_shell(None, only(&["/bin/sh"]));
        assert_eq!(sh.program, PathBuf::from("/bin/sh"));
        let relative = resolve_unix_shell(Some("zsh".into()), only(&["zsh", "/bin/sh"]));
        assert_eq!(relative.program, PathBuf::from("/bin/sh"));
        let nothing = resolve_unix_shell(Some(OsString::new()), only(&[]));
        assert_eq!(nothing.program, PathBuf::from("/bin/sh"));
    }

    #[test]
    fn windows_order_is_pwsh_powershell_cmd() {
        let found = |names: &'static [&'static str]| {
            move |name: &str| {
                names
                    .contains(&name)
                    .then(|| PathBuf::from(format!("C:\\bin\\{name}.exe")))
            }
        };
        let shell = resolve_windows_shell(found(&["pwsh", "powershell", "cmd"]));
        assert!(shell.program.to_string_lossy().ends_with("pwsh.exe"));
        assert_eq!(shell.interactive_args(), ["-NoLogo"]);
        let shell = resolve_windows_shell(found(&["powershell", "cmd"]));
        assert!(shell.program.to_string_lossy().ends_with("powershell.exe"));
        let shell = resolve_windows_shell(found(&["cmd"]));
        assert_eq!(shell.command_args("dir"), ["/C", "dir"]);
        let shell = resolve_windows_shell(found(&[]));
        assert_eq!(shell.program, PathBuf::from("cmd.exe"));
    }

    #[test]
    fn posix_command_args_end_with_the_line() {
        let shell = resolve_unix_shell(None, only(&["/bin/sh"]));
        let args = shell.command_args("echo hi | wc -c");
        assert_eq!(args.last().map(String::as_str), Some("echo hi | wc -c"));
        assert!(args.contains(&"-c".to_string()));
    }
}
