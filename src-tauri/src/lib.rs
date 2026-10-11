//! AgentFlow backend: wires the Tauri app, managed state and IPC commands.

mod agents;
mod commands;
pub mod error;
mod fs;
mod providers;
mod pty;
pub mod settings;
pub mod themes;
mod tokens;
mod util;

use std::path::Path;

use tauri::Manager;

use crate::settings::SettingsStore;
use crate::themes::ThemeStore;

const SETTINGS_FILE: &str = "settings.json";
const THEMES_DIR: &str = "themes";

/// Starts the desktop app. Called from `main.rs`.
pub fn run() {
    init_tracing();

    let result = tauri::Builder::default()
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
        ])
        .run(tauri::generate_context!());

    if let Err(err) = result {
        tracing::error!("AgentFlow stopped with an error: {err}");
        std::process::exit(1);
    }
}

fn manage_stores(app: &tauri::App, config_dir: &Path) {
    tracing::info!("using config dir {}", config_dir.display());
    app.manage(SettingsStore::load(config_dir.join(SETTINGS_FILE)));
    app.manage(ThemeStore::new(config_dir.join(THEMES_DIR)));
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
