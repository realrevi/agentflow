//! User settings: a versioned JSON file in the OS app config dir, written atomically.

mod io;
mod migrate;
mod model;
mod normalize;
mod store;

pub use io::{load_from_file, save_to_file};
pub use migrate::migrate;
pub use model::{
    AgentBold, Appearance, CharacterSettings, FilesSettings, Settings, TerminalSettings, UiState,
    CURRENT_VERSION, CURSOR_STYLES, DEFAULT_BOLD_COLOR, DEFAULT_LANGUAGE, DEFAULT_THEME_ID,
    MAX_RECENT_WORKSPACES, SIDEBAR_VIEWS, SUPPORTED_LANGUAGES,
};
pub use store::SettingsStore;
