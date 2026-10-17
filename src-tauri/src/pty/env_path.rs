//! GUI apps on macOS (and some Linux launchers) start with a minimal PATH, so CLIs installed
//! with Homebrew, npm or nvm are invisible to them. The user's login shell is asked for its
//! PATH once; it is merged with the process PATH and used to find and run CLIs.

use std::ffi::{OsStr, OsString};
use std::sync::OnceLock;

// Only the unix login-shell probe (and the tests) use the marker helpers.
#[cfg_attr(not(any(unix, test)), allow(dead_code))]
const MARKER: &str = "__AGENTFLOW_PATH__";

static SEARCH_PATH: OnceLock<Option<OsString>> = OnceLock::new();

/// Login-shell PATH entries first, then the process PATH, without duplicates. Blocking on the
/// first call (it may start a shell, bounded by a timeout); cached afterwards.
pub fn search_path() -> Option<OsString> {
    SEARCH_PATH
        .get_or_init(|| {
            let current = std::env::var_os("PATH");
            merge(
                login_shell_path().as_deref().map(OsStr::new),
                current.as_deref(),
            )
        })
        .clone()
}

/// The text between the two markers, ignoring anything rc files print around it.
#[cfg_attr(not(any(unix, test)), allow(dead_code))]
fn extract(output: &str) -> Option<&str> {
    let start = output.find(MARKER)? + MARKER.len();
    let len = output[start..].find(MARKER)?;
    Some(&output[start..start + len]).filter(|path| !path.trim().is_empty())
}

fn merge(first: Option<&OsStr>, second: Option<&OsStr>) -> Option<OsString> {
    let mut entries = Vec::new();
    for list in [first, second].into_iter().flatten() {
        for entry in std::env::split_paths(list) {
            if !entry.as_os_str().is_empty() && !entries.contains(&entry) {
                entries.push(entry);
            }
        }
    }
    if entries.is_empty() {
        return None;
    }
    std::env::join_paths(entries).ok()
}

#[cfg(unix)]
fn login_shell_path() -> Option<String> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    const TIMEOUT: Duration = Duration::from_secs(5);
    let shell = std::env::var_os("SHELL").filter(|s| std::path::Path::new(s).is_file())?;
    let script = format!("printf '{MARKER}%s{MARKER}' \"$PATH\"");
    let mut child = Command::new(shell)
        .args(["-i", "-l", "-c", &script])
        .env("DISABLE_AUTO_UPDATE", "true") // keep oh-my-zsh from updating itself
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut output = String::new();
        let _ = stdout.read_to_string(&mut output);
        let _ = tx.send(output);
    });
    let output = rx.recv_timeout(TIMEOUT);
    // Done reading (or gave up): make sure the shell is gone and reaped either way.
    let _ = child.kill();
    let _ = child.wait();
    let output = output.ok()?;
    extract(&output).map(str::to_owned)
}

#[cfg(not(unix))]
fn login_shell_path() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_between_markers() {
        let noisy = format!("motd\n{MARKER}/opt/bin:/usr/bin{MARKER}bye");
        assert_eq!(extract(&noisy), Some("/opt/bin:/usr/bin"));
        assert_eq!(extract("no markers"), None);
        assert_eq!(extract(&format!("{MARKER}  {MARKER}")), None);
    }

    #[cfg(unix)]
    #[test]
    fn merges_without_duplicates_login_first() {
        let merged = merge(
            Some(OsStr::new("/opt/homebrew/bin:/usr/bin")),
            Some(OsStr::new("/usr/bin:/bin::")),
        )
        .expect("merged");
        assert_eq!(merged, OsString::from("/opt/homebrew/bin:/usr/bin:/bin"));
        assert_eq!(merge(None, None), None);
    }

    #[cfg(unix)]
    #[test]
    fn search_path_contains_the_process_path() {
        let path = search_path().expect("PATH");
        let entries: Vec<_> = std::env::split_paths(&path).collect();
        for entry in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
            if !entry.as_os_str().is_empty() {
                assert!(entries.contains(&entry), "{entry:?} missing");
            }
        }
    }
}
