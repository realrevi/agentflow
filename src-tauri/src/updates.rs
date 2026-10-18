//! App updates (phase 12) through `tauri-plugin-updater`: `latest.json` on the newest
//! published GitHub release, signed with the updater key (the public key is in
//! tauri.conf.json, so a tampered download is rejected). Checking never installs: the
//! frontend asks, and only an explicit "Install and restart" downloads and installs.

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::AppHandle;
use tauri_plugin_updater::{Error as UpdaterError, Update, UpdaterExt};

use crate::error::{AppError, AppResult};
use crate::util::lock;

/// A newer release than the running one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    /// The release notes from latest.json (markdown), if any.
    pub notes: Option<String>,
    /// RFC 3339.
    pub date: Option<String>,
}

/// Download progress, sent on the install channel; the app restarts after `Finished`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UpdateProgress {
    Started { total: Option<u64> },
    Progress { downloaded: u64 },
    Finished,
}

/// The update found by the last check, kept for `install`.
#[derive(Default)]
pub struct Updates {
    pending: std::sync::Mutex<Option<Update>>,
}

impl Updates {
    pub async fn check(&self, app: &AppHandle) -> AppResult<Option<UpdateInfo>> {
        let updater = app.updater().map_err(updater_error)?;
        let found = match updater.check().await {
            Ok(found) => found,
            // No published release (yet): nothing newer to offer.
            Err(UpdaterError::ReleaseNotFound) => None,
            Err(err) => return Err(updater_error(err)),
        };
        let info = found.as_ref().map(info_of);
        *lock(&self.pending) = found;
        Ok(info)
    }

    /// Downloads the pending update, verifies its signature, installs it and restarts.
    pub async fn install(
        &self,
        app: &AppHandle,
        progress: Channel<UpdateProgress>,
    ) -> AppResult<()> {
        let update = lock(&self.pending)
            .take()
            .ok_or_else(|| AppError::NotFound("no update to install, check again".into()))?;
        let mut downloaded = 0_u64;
        let mut started = false;
        let finished = progress.clone();
        update
            .download_and_install(
                |chunk, total| {
                    if !started {
                        started = true;
                        let _ = progress.send(UpdateProgress::Started { total });
                    }
                    downloaded += chunk as u64;
                    let _ = progress.send(UpdateProgress::Progress { downloaded });
                },
                move || {
                    let _ = finished.send(UpdateProgress::Finished);
                },
            )
            .await
            .map_err(updater_error)?;
        tracing::info!("update {} installed, restarting", update.version);
        app.restart()
    }
}

fn info_of(update: &Update) -> UpdateInfo {
    UpdateInfo {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        notes: update.body.clone().filter(|body| !body.trim().is_empty()),
        date: update
            .raw_json
            .get("pub_date")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    }
}

fn updater_error(err: UpdaterError) -> AppError {
    tracing::warn!("updater: {err}");
    AppError::Internal(format!("update failed: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_events_serialize_for_the_frontend() {
        let json = |event| serde_json::to_value(event).expect("json");
        assert_eq!(
            json(UpdateProgress::Started { total: Some(10) }),
            serde_json::json!({ "type": "started", "total": 10 })
        );
        assert_eq!(
            json(UpdateProgress::Progress { downloaded: 4 }),
            serde_json::json!({ "type": "progress", "downloaded": 4 })
        );
        assert_eq!(
            json(UpdateProgress::Finished),
            serde_json::json!({ "type": "finished" })
        );
    }
}
