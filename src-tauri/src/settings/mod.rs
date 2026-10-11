//! User settings: a versioned JSON file in the OS app config dir, written atomically.

mod io;
mod migrate;
mod model;
mod normalize;
mod store;

pub use io::{load_from_file, save_to_file};
pub use migrate::migrate;
pub use model::{
    AgentBold, Appearance, Settings, UiState, CURRENT_VERSION, DEFAULT_BOLD_COLOR,
    DEFAULT_LANGUAGE, DEFAULT_THEME_ID, SIDEBAR_VIEWS, SUPPORTED_LANGUAGES,
};
pub use store::SettingsStore;
