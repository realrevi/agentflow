//! AgentFlow backend: wires the Tauri app, managed state and IPC commands.

mod agents;
mod commands;
pub mod error;
mod fs;
mod providers;
pub mod pty;
pub mod settings;
pub mod themes;
mod tokens;
mod util;

use std::path::Path;

use tauri::webview::PageLoadEvent;
use tauri::{Manager, RunEvent};

use crate::fs::FsState;
use crate::pty::{CliDetector, PtyManager};
use crate::settings::SettingsStore;
use crate::themes::ThemeStore;

const SETTINGS_FILE: &str = "settings.json";
const THEMES_DIR: &str = "themes";

/// Starts the desktop app. Called from `main.rs`.
pub fn run() {
    init_tracing();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .on_page_load(|webview, payload| {
            // A reload drops every terminal channel, so the old sessions can never be shown
            // again: stop them instead of leaving shells and agent CLIs running unseen.
            if payload.event() == PageLoadEvent::Started {
                if let Some(manager) = webview.try_state::<PtyManager>() {
                    manager.kill_all();
                }
            }
        })
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            manage_stores(app, &config_dir);
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
        }
    });
}

fn manage_stores(app: &tauri::App, config_dir: &Path) {
    tracing::info!("using config dir {}", config_dir.display());
    app.manage(SettingsStore::load(config_dir.join(SETTINGS_FILE)));
    app.manage(ThemeStore::new(config_dir.join(THEMES_DIR)));
    app.manage(PtyManager::new());
    app.manage(CliDetector::new());
    app.manage(FsState::new());
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
