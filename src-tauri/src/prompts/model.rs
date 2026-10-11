//! The on-disk shape of one workspace's prompt history.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptEntry {
    pub text: String,
    /// When it was sent (ms since the Unix epoch).
    pub at: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WorkspaceHistory {
    pub version: u32,
    /// The workspace folder (informational; the file name is derived from it).
    pub workspace: Option<String>,
    /// Oldest first, per panel key.
    pub panels: BTreeMap<String, Vec<PromptEntry>>,
}
