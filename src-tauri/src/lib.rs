//! AgentFlow backend: wires the Tauri app, managed state and IPC commands.

mod agents;
pub mod chat;
mod commands;
pub mod conversations;
pub mod error;
mod fs;
pub mod prompts;
pub mod providers;
pub mod pty;
pub mod settings;
pub mod themes;
mod tokens;
mod util;

use std::path::Path;

use tauri::webview::PageLoadEvent;
use tauri::{Manager, RunEvent};

use crate::chat::ChatManager;
use crate::conversations::ConversationStore;
use crate::fs::FsState;
use crate::prompts::PromptHistory;
use crate::providers::{KeyVault, OsKeychain};
use crate::pty::{CliDetector, PtyManager};
use crate::settings::{Settings, SettingsStore};
use crate::themes::ThemeStore;
use crate::tokens::{CliRoots, UsageTracker};

const SETTINGS_FILE: &str = "settings.json";
const THEMES_DIR: &str = "themes";
const PROMPT_HISTORY_DIR: &str = "prompt-history";
const CONVERSATIONS_DIR: &str = "conversations";
const USAGE_DIR: &str = "usage";

/// Starts the desktop app. Called from `main.rs`.
pub fn run() {
    init_tracing();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
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
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_data_dir()?;
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
    start_usage(app, &snapshot, data_dir);
}

/// The usage tracker is managed at once; the ledger, backfill and CLI logs load in the
/// background, so startup never waits for them.
fn start_usage(app: &tauri::App, settings: &Settings, data_dir: &Path) {
    let roots = CliRoots::detect(app.path().home_dir().ok().as_deref());
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
