//! Theme storage and validation. Accepts VS Code color theme JSON (JSONC allowed);
//! the frontend maps the validated `colors` to `--af-*` CSS variables. Editor syntax colors
//! derived from `tokenColors` are added to `colors` as `agentflow.syntax.*`.

mod builtin;
mod jsonc;
mod model;
mod parse;
mod store;
mod syntax;

pub use jsonc::strip_jsonc;
pub use model::{ThemeData, ThemeKind, ThemeSummary};
pub use parse::{parse_theme, ParsedTheme};
pub use store::ThemeStore;
