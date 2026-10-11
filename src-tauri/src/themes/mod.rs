//! Theme storage and validation. Accepts VS Code color theme JSON (JSONC allowed);
//! the frontend maps the validated `colors` to `--af-*` CSS variables.

mod builtin;
mod jsonc;
mod model;
mod parse;
mod store;

pub use jsonc::strip_jsonc;
pub use model::{ThemeData, ThemeKind, ThemeSummary};
pub use parse::{parse_theme, ParsedTheme};
pub use store::ThemeStore;
