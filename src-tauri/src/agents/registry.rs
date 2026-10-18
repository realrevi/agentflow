//! `.agentflow/installed.json`: what the installer wrote, so updates and uninstalls only
//! touch our files and notice edits (content hashes).

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::targets::{Scope, Target};
use super::template::{Answers, Template};
use crate::error::AppResult;
use crate::util::write_atomic;

pub const REGISTRY_PATH: &str = ".agentflow/installed.json";
const REGISTRY_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    File,
    Section,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledFile {
    pub target: Target,
    pub scope: Scope,
    pub path: String,
    pub kind: FileKind,
    /// SHA-256 of the file, or of the section's content (LF line endings).
    pub hash: String,
    /// The shared file did not exist before (an emptied one goes to the trash on uninstall).
    #[serde(default)]
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    pub id: String,
    pub template: Template,
    pub answers: Answers,
    pub targets: Vec<Target>,
    pub scope: Scope,
    pub installed_at: u64,
    pub updated_at: u64,
    pub files: Vec<InstalledFile>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    pub version: u32,
    pub agents: Vec<Installed>,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            version: REGISTRY_VERSION,
            agents: Vec::new(),
        }
    }
}

pub fn hash(text: &str) -> String {
    hash_bytes(text.as_bytes())
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

impl Registry {
    /// A missing or unreadable registry is empty (it is rebuilt by the next install).
    pub fn load(root: &Path) -> Self {
        std::fs::read_to_string(root.join(REGISTRY_PATH))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, root: &Path) -> AppResult<()> {
        let json = serde_json::to_vec_pretty(self)?;
        write_atomic(&root.join(REGISTRY_PATH), &json)?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Installed> {
        self.agents.iter().find(|agent| agent.id == id)
    }

    pub fn put(&mut self, entry: Installed) {
        self.agents.retain(|agent| agent.id != entry.id);
        self.agents.push(entry);
        self.agents.sort_by(|a, b| a.id.cmp(&b.id));
    }

    pub fn remove(&mut self, id: &str) {
        self.agents.retain(|agent| agent.id != id);
    }
}
