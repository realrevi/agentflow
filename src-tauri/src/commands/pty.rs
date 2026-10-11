use std::path::{Path, PathBuf};

use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Manager, State};

use crate::error::{AppError, AppResult};
use crate::pty::{
    resolve_launch, CliDetector, CliStatus, ExitInfo, OutputSink, PtyKind, PtyManager, PtyOutput,
    SessionId, SessionInfo, TermSize,
};
use crate::util::run_blocking;

/// Starts a terminal. Output arrives on `on_output` as raw bytes (`ArrayBuffer` in JS),
/// followed by one JSON message `{ "type": "exit", "code", "signal" }`.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // mirrors the frontend call one-to-one
pub async fn pty_spawn(
    app: AppHandle,
    manager: State<'_, PtyManager>,
    kind: PtyKind,
    command: Option<String>,
    args: Option<Vec<String>>,
    cwd: Option<String>,
    cols: u16,
    rows: u16,
    on_output: Channel<InvokeResponseBody>,
) -> AppResult<SessionInfo> {
    let home = app.path().home_dir().ok();
    let manager = manager.inner().clone();
    run_blocking(move || {
        let cwd = resolve_cwd(cwd.as_deref(), home)?;
        let launch = resolve_launch(kind, command.as_deref(), &args.unwrap_or_default())?;
        let size = TermSize::clamped(cols, rows);
        manager.spawn(kind, &launch, &cwd, size, channel_sink(on_output))
    })
    .await
}

#[tauri::command]
pub async fn pty_write(
    manager: State<'_, PtyManager>,
    id: SessionId,
    data: String,
) -> AppResult<()> {
    let manager = manager.inner().clone();
    run_blocking(move || manager.write(id, data.as_bytes())).await
}

#[tauri::command]
pub async fn pty_resize(
    manager: State<'_, PtyManager>,
    id: SessionId,
    cols: u16,
    rows: u16,
) -> AppResult<()> {
    let manager = manager.inner().clone();
    run_blocking(move || manager.resize(id, TermSize::clamped(cols, rows))).await
}

/// Returns `false` if the session had already ended.
#[tauri::command]
pub async fn pty_kill(manager: State<'_, PtyManager>, id: SessionId) -> AppResult<bool> {
    let manager = manager.inner().clone();
    run_blocking(move || Ok(manager.kill(id))).await
}

#[tauri::command]
pub async fn pty_list(manager: State<'_, PtyManager>) -> AppResult<Vec<SessionInfo>> {
    Ok(manager.list())
}

/// Agent CLIs on PATH. Cached; pass `refresh: true` to look again.
#[tauri::command]
pub async fn cli_detect(
    detector: State<'_, CliDetector>,
    refresh: Option<bool>,
) -> AppResult<Vec<CliStatus>> {
    let detector = detector.inner().clone();
    run_blocking(move || Ok(detector.detect(refresh.unwrap_or(false)))).await
}

/// The requested folder (which must exist), or the home directory when none is given.
fn resolve_cwd(cwd: Option<&str>, home: Option<PathBuf>) -> AppResult<PathBuf> {
    // Not trimmed: leading/trailing spaces are legal in folder names.
    match cwd.filter(|c| !c.trim().is_empty()) {
        Some(dir) if Path::new(dir).is_dir() => Ok(PathBuf::from(dir)),
        Some(dir) => Err(AppError::InvalidInput(format!(
            "folder does not exist: {dir}"
        ))),
        None => home.ok_or_else(|| AppError::NotFound("home directory".into())),
    }
}

fn channel_sink(channel: Channel<InvokeResponseBody>) -> OutputSink {
    Box::new(move |output| {
        let body = match output {
            PtyOutput::Data(bytes) => InvokeResponseBody::Raw(bytes),
            PtyOutput::Exit(info) => InvokeResponseBody::Json(exit_message(&info)),
        };
        // The panel may already be closed; nothing to do then.
        let _ = channel.send(body);
    })
}

fn exit_message(info: &ExitInfo) -> String {
    serde_json::json!({ "type": "exit", "code": info.code, "signal": info.signal }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cwd_defaults_to_home_and_must_exist() {
        let home = std::env::temp_dir();
        assert_eq!(resolve_cwd(None, Some(home.clone())).expect("home"), home);
        assert_eq!(
            resolve_cwd(Some("  "), Some(home.clone())).expect("home"),
            home
        );
        let dir = home.to_string_lossy().into_owned();
        assert_eq!(resolve_cwd(Some(&dir), None).expect("dir"), home);
        assert!(matches!(
            resolve_cwd(Some("/definitely/not/here"), None),
            Err(AppError::InvalidInput(_))
        ));
        assert!(resolve_cwd(None, None).is_err());
    }

    #[test]
    fn exit_message_shape() {
        let message = exit_message(&ExitInfo {
            code: Some(2),
            signal: None,
        });
        let value: serde_json::Value = serde_json::from_str(&message).expect("json");
        assert_eq!(value["type"], "exit");
        assert_eq!(value["code"], 2);
        assert!(value["signal"].is_null());
    }
}
