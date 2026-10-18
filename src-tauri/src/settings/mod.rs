//! User settings: a versioned JSON file in the OS app config dir, written atomically.

mod agent;
mod chat;
mod io;
mod library;
mod migrate;
mod model;
mod normalize;
mod onboarding;
mod providers;
mod store;
mod usage;
mod workstreams;

pub use agent::{AgentSettings, ModelTools};
pub use chat::{ChatSettings, ModelWindow, DEFAULT_CONTEXT_WINDOW};
pub use io::{load_from_file, save_to_file};
pub use library::{CustomSource, LibrarySettings, MAX_CUSTOM_SOURCES};
pub use migrate::migrate;
pub use model::{
    AgentBold, Appearance, CharacterSettings, FilesSettings, Settings, TerminalSettings, UiState,
    CURRENT_VERSION, CURSOR_STYLES, DEFAULT_BOLD_COLOR, DEFAULT_LANGUAGE, DEFAULT_THEME_ID,
    MAX_RECENT_WORKSPACES, SIDEBAR_VIEWS, SUPPORTED_LANGUAGES,
};
pub use onboarding::OnboardingSettings;
pub use providers::{
    is_valid_provider_id, ProviderEntry, ANTHROPIC_PRESET, OPENAI_PRESETS, PROVIDER_KINDS,
};
pub use store::SettingsStore;
pub use usage::{Budgets, PriceOverride, ProviderBudget, UsageSettings};
pub use workstreams::{HandoffMode, WorkstreamSettings};
