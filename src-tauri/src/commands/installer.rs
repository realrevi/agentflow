//! The agent installer (phase 8): starters, template import/export and validation,
//! workspace detection, plan/install/uninstall, installed AgentFlow agents, and "Create with
//! AI". All file I/O runs off the async runtime; writes go through the workspace sandbox
//! (user scope only into the documented home folders, after its own confirmation).

use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager, State};

use super::chat::deps;
use crate::agent::AgentServices;
use crate::agents::{
    self, AgentProfile, Confirmations, Detection, GenerateRequest, GenerateResult, InstallRequest,
    Installed, Plan, Registry, Roots, Template,
};
use crate::chat::ChatManager;
use crate::conversations::{Conversation, ConversationPatch, ConversationStore};
use crate::error::{AppError, AppResult};
use crate::fs::move_to_trash;
use crate::providers::KeyVault;
use crate::settings::SettingsStore;
use crate::tokens::UsageTracker;
use crate::util::run_blocking;

/// Imported template files larger than this are refused.
const MAX_TEMPLATE_FILE: u64 = 512 * 1024;

fn workspace(path: Option<String>) -> AppResult<PathBuf> {
    path.map(PathBuf::from)
        .ok_or_else(|| AppError::InvalidInput("open a workspace folder first".into()))
}

fn roots(app: &AppHandle, workspace: &std::path::Path) -> AppResult<Roots> {
    Roots::new(workspace, app.path().home_dir().ok())
}

#[tauri::command]
pub async fn agents_starters() -> AppResult<Vec<Template>> {
    Ok(agents::starters())
}

/// Problems with a template (empty = valid); JSON that does not even parse is one problem.
#[tauri::command]
pub async fn agents_validate(template: Value) -> AppResult<Vec<String>> {
    Ok(match serde_json::from_value::<Template>(template) {
        Ok(template) => agents::validate(&template),
        Err(err) => vec![format!("the template does not match the format: {err}")],
    })
}

#[tauri::command]
pub async fn agents_template_read(path: String) -> AppResult<Template> {
    run_blocking(move || {
        let meta = std::fs::metadata(&path)?;
        if meta.len() > MAX_TEMPLATE_FILE {
            return Err(AppError::InvalidInput(
                "the file is too large for a template".into(),
            ));
        }
        let template: Template = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
        let errors = agents::validate(&template);
        if errors.is_empty() {
            Ok(template)
        } else {
            Err(AppError::InvalidInput(errors.join("; ")))
        }
    })
    .await
}

/// Export: the user picked `path` in a save dialog.
#[tauri::command]
pub async fn agents_template_write(path: String, template: Template) -> AppResult<()> {
    run_blocking(move || {
        let json = serde_json::to_string_pretty(&template)?;
        crate::util::write_atomic(std::path::Path::new(&path), format!("{json}\n").as_bytes())?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn agents_detect(workspace_path: Option<String>) -> AppResult<Detection> {
    let root = workspace(workspace_path)?;
    run_blocking(move || Ok(agents::detect(&std::fs::canonicalize(&root)?))).await
}

#[tauri::command]
pub async fn agents_plan(
    app: AppHandle,
    workspace_path: Option<String>,
    request: InstallRequest,
) -> AppResult<Plan> {
    let root = workspace(workspace_path)?;
    run_blocking(move || {
        let roots = roots(&app, &root)?;
        let registry = Registry::load(roots.workspace.path());
        agents::plan(&roots, &registry, &request)
    })
    .await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub plan: Plan,
    pub installed: Installed,
}

#[tauri::command]
pub async fn agents_install(
    app: AppHandle,
    workspace_path: Option<String>,
    request: InstallRequest,
    confirm: Confirmations,
) -> AppResult<InstallResult> {
    let root = workspace(workspace_path)?;
    run_blocking(move || {
        let roots = roots(&app, &root)?;
        let (plan, installed) = agents::install(&roots, &request, &confirm, move_to_trash)?;
        Ok(InstallResult { plan, installed })
    })
    .await
}

#[tauri::command]
pub async fn agents_installed(workspace_path: Option<String>) -> AppResult<Vec<Installed>> {
    let root = workspace(workspace_path)?;
    run_blocking(move || Ok(Registry::load(&root).agents)).await
}

#[tauri::command]
pub async fn agents_uninstall_plan(
    app: AppHandle,
    workspace_path: Option<String>,
    id: String,
) -> AppResult<Plan> {
    let root = workspace(workspace_path)?;
    run_blocking(move || agents::uninstall_plan(&roots(&app, &root)?, &id)).await
}

#[tauri::command]
pub async fn agents_uninstall(
    app: AppHandle,
    workspace_path: Option<String>,
    id: String,
    confirm: Confirmations,
    keep: Vec<String>,
) -> AppResult<Plan> {
    let root = workspace(workspace_path)?;
    run_blocking(move || {
        agents::uninstall(&roots(&app, &root)?, &id, &confirm, &keep, move_to_trash)
    })
    .await
}

#[tauri::command]
pub async fn agents_profiles(workspace_path: Option<String>) -> AppResult<Vec<AgentProfile>> {
    let Some(root) = workspace_path.map(PathBuf::from) else {
        return Ok(Vec::new());
    };
    run_blocking(move || Ok(agents::list_profiles(&root))).await
}

/// A new API chat with an installed AgentFlow agent: its role, tool allowlist and (never
/// looser than the user's default) permission mode are stored with the conversation.
#[tauri::command]
pub async fn agents_profile_chat(
    settings: State<'_, SettingsStore>,
    store: State<'_, ConversationStore>,
    workspace_path: Option<String>,
    profile_id: String,
    provider_id: String,
    model: String,
) -> AppResult<Conversation> {
    let default_mode = settings.get().await.agent.default_mode;
    let store = store.inner().clone();
    run_blocking(move || {
        let root = workspace(workspace_path.clone())?;
        let profile = agents::get_profile(&root, &profile_id)?;
        let ws = workspace_path.as_deref();
        let created = store.create(ws, Some(provider_id), &model, "")?;
        let patch = ConversationPatch {
            title: Some(profile.name.clone()),
            ..ConversationPatch::default()
        };
        store.update(ws, &created.id, patch)?;
        store.update_agent(ws, &created.id, |agent| {
            agent.permission_mode = profile.permission_mode.map(|m| m.stricter(default_mode));
            agent.profile = Some(profile.snapshot());
        })?;
        store.get(ws, &created.id)
    })
    .await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // managed state, one argument each
pub async fn agents_generate(
    settings: State<'_, SettingsStore>,
    store: State<'_, ConversationStore>,
    vault: State<'_, KeyVault>,
    manager: State<'_, ChatManager>,
    tracker: State<'_, UsageTracker>,
    agent: State<'_, AgentServices>,
    request: GenerateRequest,
) -> AppResult<GenerateResult> {
    let deps = deps(&settings, &store, &vault, &manager, &tracker, &agent).await;
    agents::generate(deps, request).await
}
