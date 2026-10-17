//! run_command: the user's shell, in the workspace, never interactive (stdin closed),
//! stdout + stderr + exit code captured, killed (with its child processes) on timeout or
//! cancel.

use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

use super::args::{optional_u64, string};
use super::{ToolEnv, ToolMeta, ToolOutput};
use crate::providers::CancelToken;
use crate::pty::{default_shell, search_path};

/// Output beyond this is drained but not kept (the cap cuts it further for the model).
const KEEP_BYTES: usize = 512 * 1024;

pub async fn run_command(env: &ToolEnv, input: &Value, cancel: &CancelToken) -> ToolOutput {
    let line = match string(input, "command") {
        Ok(line) if !line.trim().is_empty() => line,
        Ok(_) => return ToolOutput::error("The command is empty."),
        Err(err) => return ToolOutput::error(err),
    };
    let timeout = optional_u64(input, "timeout")
        .map(Duration::from_secs)
        .unwrap_or(env.timeout)
        .clamp(Duration::from_secs(1), Duration::from_secs(3_600));
    run(env.root.path(), &line, timeout, cancel).await
}

pub async fn run(
    dir: &std::path::Path,
    line: &str,
    timeout: Duration,
    cancel: &CancelToken,
) -> ToolOutput {
    let shell = default_shell();
    let mut command = Command::new(&shell.program);
    command
        .args(shell.batch_args(&utf8_line(&shell.name(), line)))
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(path) = search_path() {
        command.env("PATH", path);
    }
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    #[cfg(unix)]
    command.process_group(0); // its own group, so the whole tree can be killed
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => return ToolOutput::error(format!("Could not start the shell: {err}")),
    };
    let pid = child.id();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let reading = async { tokio::join!(drain(stdout), drain(stderr)) };
    tokio::pin!(reading);
    let outcome = tokio::select! {
        biased;
        () = cancel.cancelled() => Ended::Cancelled,
        () = tokio::time::sleep(timeout) => Ended::TimedOut,
        status = async {
            let output = (&mut reading).await;
            (child.wait().await, output)
        } => Ended::Exited(status.0.ok().and_then(|s| s.code()), status.1),
    };
    match outcome {
        Ended::Exited(code, (out, err)) => {
            let code_text = code.map_or_else(|| "killed".to_owned(), |c| c.to_string());
            let content = format!(
                "exit code: {code_text}\n--- stdout ---\n{}\n--- stderr ---\n{}",
                tidy(&out),
                tidy(&err)
            );
            ToolOutput {
                content,
                is_error: code != Some(0),
                meta: ToolMeta {
                    exit_code: code.map(i64::from),
                    ..ToolMeta::default()
                },
            }
        }
        Ended::Cancelled | Ended::TimedOut => {
            kill_tree(pid);
            let _ = child.kill().await;
            if matches!(outcome, Ended::Cancelled) {
                ToolOutput::error("The command was stopped (the turn was cancelled).")
            } else {
                ToolOutput::error(format!(
                    "The command timed out after {} s and was stopped.",
                    timeout.as_secs()
                ))
            }
        }
    }
}

enum Ended {
    Exited(Option<i32>, (String, String)),
    Cancelled,
    TimedOut,
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Windows consoles write in the OEM code page by default; ask for UTF-8 output.
fn utf8_line(shell: &str, line: &str) -> String {
    if !cfg!(windows) {
        return line.to_owned();
    }
    match shell.to_ascii_lowercase().as_str() {
        "pwsh" | "powershell" => format!(
            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $OutputEncoding = \
             [System.Text.Encoding]::UTF8; {line}"
        ),
        "cmd" => format!("chcp 65001 >nul & {line}"),
        _ => line.to_owned(),
    }
}

async fn drain(stream: Option<impl AsyncRead + Unpin>) -> String {
    let Some(mut stream) = stream else {
        return String::new();
    };
    let mut kept = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let room = KEEP_BYTES.saturating_sub(kept.len());
                kept.extend_from_slice(&buffer[..n.min(room)]);
            }
        }
    }
    String::from_utf8_lossy(&kept).into_owned()
}

/// Kills the shell and everything it started.
fn kill_tree(pid: Option<u32>) {
    let Some(pid) = pid else { return };
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
    #[cfg(unix)]
    {
        if let Ok(group) = i32::try_from(pid) {
            // SAFETY: kill(2) with a negative pid signals that process group; no memory
            // is shared with the call.
            unsafe {
                libc::kill(-group, libc::SIGKILL);
            }
        }
    }
}

#[cfg(windows)]
use std::os::windows::process::CommandExt as _;

/// Trailing spaces per line (PowerShell pads tables to the console width) cost tokens only.
fn tidy(output: &str) -> String {
    output
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}
