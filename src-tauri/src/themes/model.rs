//! Theme types sent to the frontend (`ThemeSummary` / `ThemeData` in `src/shared/types.ts`).

use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeKind {
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeSummary {
    pub id: String,
    pub name: String,
    pub kind: ThemeKind,
    pub builtin: bool,
}

/// A theme ready to apply. `tokenColors` are not sent; the editor syntax colors derived
/// from them arrive in `colors` as `agentflow.syntax.*`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeData {
    #[serde(flatten)]
    pub summary: ThemeSummary,
    pub colors: BTreeMap<String, String>,
}
