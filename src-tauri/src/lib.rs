//! AgentFlow backend: wires the Tauri app, managed state and IPC commands.

mod agent;
mod agents;
pub mod chat;
mod commands;
pub mod conversations;
mod crash;
pub mod error;
mod fs;
mod library;
pub mod prompts;
pub mod providers;
pub mod pty;
pub mod settings;
pub mod themes;
mod tokens;
mod turns;
mod updates;
mod util;
mod workstreams;

use std::path::Path;

use tauri::webview::PageLoadEvent;
use tauri::{Manager, RunEvent};

use crate::agent::{AgentServices, CheckpointStore, ModelCapabilities, PermissionGate};
use crate::chat::ChatManager;
use crate::conversations::ConversationStore;
use crate::fs::FsState;
use crate::library::Library;
use crate::prompts::PromptHistory;
use crate::providers::{KeyVault, OsKeychain};
use crate::pty::{CliDetector, PtyManager};
use crate::settings::{Settings, SettingsStore};
use crate::themes::ThemeStore;
use crate::tokens::{CliRoots, UsageTracker};
use crate::turns::TurnWatcher;
use crate::updates::Updates;
use crate::workstreams::WorkstreamLog;

const SETTINGS_FILE: &str = "settings.json";
const THEMES_DIR: &str = "themes";
const PROMPT_HISTORY_DIR: &str = "prompt-history";
const CONVERSATIONS_DIR: &str = "conversations";
const USAGE_DIR: &str = "usage";
const CHECKPOINTS_DIR: &str = "checkpoints";
const MODEL_TOOLS_FILE: &str = "model-tools.json";
const WORKSTREAMS_DIR: &str = "workstreams";
const LIBRARY_DIR: &str = "library";

/// Starts the desktop app. Called from `main.rs`.
pub fn run() {
    init_tracing();
    // reqwest (ours and the updater's) is built without a bundled crypto provider: ring.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_page_load(|webview, payload| {
            // A reload drops every terminal channel, so the old sessions can never be shown
            // again: stop them instead of leaving shells and agent CLIs running unseen.
            if payload.event() == PageLoadEvent::Started {
                if let Some(manager) = webview.try_state::<PtyManager>() {
                    manager.kill_all();
                }
                if let Some(chats) = webview.try_state::<ChatManager>() {
                    chats.cancel_all();
                }
            }
        })
        .setup(|app| {
            let (config_dir, data_dir) = match dev_data_dir() {
                Some(dir) => (dir.join("config"), dir.join("data")),
                None => (app.path().app_config_dir()?, app.path().app_data_dir()?),
            };
            crash::install_hook(&data_dir);
            manage_stores(app, &config_dir, &data_dir);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::themes::list_themes,
            commands::themes::get_theme,
            commands::themes::import_theme,
            commands::pty::pty_spawn,
            commands::pty::pty_write,
            commands::pty::pty_ack,
            commands::pty::pty_resize,
            commands::pty::pty_kill,
            commands::pty::pty_list,
            commands::pty::cli_detect,
            commands::fs::fs_open_workspace,
            commands::fs::fs_close_workspace,
            commands::fs::fs_list,
            commands::fs::fs_read,
            commands::fs::fs_write,
            commands::fs::fs_create,
            commands::fs::fs_rename,
            commands::fs::fs_delete,
            commands::fs::fs_quick_open,
            commands::prompts::prompts_list,
            commands::prompts::prompts_add,
            commands::conversations::conversations_list,
            commands::conversations::conversation_get,
            commands::conversations::conversation_create,
            commands::conversations::conversation_update,
            commands::conversations::conversation_delete,
            commands::chat::chat_send,
            commands::chat::chat_cancel,
            commands::chat::chat_compact,
            commands::agent::agent_permission_answer,
            commands::agent::agent_checkpoints,
            commands::agent::agent_checkpoint_restore,
            commands::installer::agents_starters,
            commands::installer::agents_validate,
            commands::installer::agents_template_read,
            commands::installer::agents_template_write,
            commands::installer::agents_detect,
            commands::installer::agents_plan,
            commands::installer::agents_install,
            commands::installer::agents_installed,
            commands::installer::agents_uninstall_plan,
            commands::installer::agents_uninstall,
            commands::installer::agents_profiles,
            commands::installer::agents_profile_chat,
            commands::installer::agents_generate,
            commands::library::library_catalog,
            commands::library::library_refresh,
            commands::library::library_detail,
            commands::library::library_token_status,
            commands::library::library_set_token,
            commands::library::library_delete_token,
            commands::providers::provider_key_status,
            commands::providers::provider_set_key,
            commands::providers::provider_delete_key,
            commands::providers::provider_models,
            commands::providers::provider_test,
            commands::usage::usage_subscribe,
            commands::usage::usage_summary,
            commands::usage::usage_panel_totals,
            commands::usage::usage_estimate,
            commands::usage::usage_budgets,
            commands::usage::usage_allow_once,
            commands::usage::usage_prices,
            commands::usage::usage_set_prices,
            commands::usage::usage_cli_status,
            commands::turns::turns_subscribe,
            commands::workstreams::workstream_log_append,
            commands::workstreams::workstream_log_read,
            commands::updates::update_check,
            commands::updates::update_install,
        ])
        .build(tauri::generate_context!());

    let app = match app {
        Ok(app) => app,
        Err(err) => {
            tracing::error!("AgentFlow failed to start: {err}");
            std::process::exit(1);
        }
    };
    app.run(|handle, event| {
        // Never leave terminal processes or the file watcher behind when the app quits.
        if let RunEvent::Exit = event {
            if let Some(fs) = handle.try_state::<FsState>() {
                fs.close();
            }
            if let Some(manager) = handle.try_state::<PtyManager>() {
                manager.shutdown();
            }
            if let Some(chats) = handle.try_state::<ChatManager>() {
                chats.cancel_all();
            }
            if let Some(tracker) = handle.try_state::<UsageTracker>() {
                tracker.shutdown();
            }
            if let Some(turns) = handle.try_state::<TurnWatcher>() {
                turns.shutdown();
            }
        }
    });
}

fn manage_stores(app: &tauri::App, config_dir: &Path, data_dir: &Path) {
    tracing::info!("using config dir {}", config_dir.display());
    let settings = SettingsStore::load(config_dir.join(SETTINGS_FILE));
    let snapshot = settings.snapshot();
    app.manage(settings);
    app.manage(ThemeStore::new(config_dir.join(THEMES_DIR)));
    app.manage(PtyManager::new());
    app.manage(CliDetector::new());
    app.manage(FsState::new());
    app.manage(PromptHistory::new(data_dir.join(PROMPT_HISTORY_DIR)));
    app.manage(ConversationStore::new(data_dir.join(CONVERSATIONS_DIR)));
    app.manage(KeyVault::new(std::sync::Arc::new(OsKeychain)));
    app.manage(ChatManager::new());
    app.manage(WorkstreamLog::new(data_dir.join(WORKSTREAMS_DIR)));
    app.manage(Library::new(data_dir.join(LIBRARY_DIR)));
    app.manage(Updates::default());
    app.manage(AgentServices {
        gate: PermissionGate::new(),
        checkpoints: CheckpointStore::new(data_dir.join(CHECKPOINTS_DIR)),
        capabilities: ModelCapabilities::load(data_dir.join(MODEL_TOOLS_FILE)),
    });
    start_usage(app, &snapshot, data_dir);
}

/// The usage tracker is managed at once; the ledger, backfill and CLI logs load in the
/// background, so startup never waits for them.
fn start_usage(app: &tauri::App, settings: &Settings, data_dir: &Path) {
    let roots = CliRoots::detect(app.path().home_dir().ok().as_deref());
    app.manage(TurnWatcher::new(roots.clone()));
    let tracker = UsageTracker::new(data_dir.join(USAGE_DIR), roots);
    app.manage(tracker.clone());
    let (settings, conversations) = (settings.clone(), data_dir.join(CONVERSATIONS_DIR));
    let spawned = std::thread::Builder::new()
        .name("agentflow-usage-start".into())
        .spawn(move || tracker.start(&settings, &conversations));
    if let Err(err) = spawned {
        tracing::warn!("usage tracking did not start: {err}");
    }
}

/// Debug builds only: `AGENTFLOW_DATA_DIR` keeps settings and app data in that folder
/// instead of the user's, e.g. for demo screenshots or manual tests on throwaway data.
fn dev_data_dir() -> Option<std::path::PathBuf> {
    if !cfg!(debug_assertions) {
        return None;
    }
    let dir = std::env::var_os("AGENTFLOW_DATA_DIR").filter(|value| !value.is_empty())?;
    Some(std::path::PathBuf::from(dir))
}

fn init_tracing() {
    let level = if cfg!(debug_assertions) {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    // `try_init` instead of `init`: never panic if a subscriber is already installed.
    let _ = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .try_init();
}
