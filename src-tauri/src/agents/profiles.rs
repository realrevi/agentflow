//! Installed AgentFlow agents: `.agentflow/agents/<id>.md` in the workspace, YAML
//! frontmatter (name, description, provider, model, permissionMode, tools, character) and the
//! instructions as the body. The reader is lenient (users may edit these files): unknown keys
//! are ignored, a broken file is skipped.

use std::path::Path;

use serde::Serialize;

use crate::agent::tools::TOOL_NAMES;
use crate::conversations::PermissionMode;
use crate::error::{AppError, AppResult};

pub const PROFILES_DIR: &str = ".agentflow/agents";
const MAX_PROFILES: usize = 100;
const MAX_SIZE: u64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProfile {
    /// The file name without `.md`.
    pub id: String,
    pub name: String,
    pub description: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub permission_mode: Option<PermissionMode>,
    /// Only these AgentFlow tools (within what the permission mode allows).
    pub tools: Option<Vec<String>>,
    pub character: Option<String>,
    pub instructions: String,
}

impl AgentProfile {
    /// What a chat keeps of this agent.
    pub fn snapshot(&self) -> crate::conversations::ProfileSnapshot {
        crate::conversations::ProfileSnapshot {
            id: self.id.clone(),
            name: self.name.clone(),
            instructions: self.instructions.clone(),
            tools: self.tools.clone(),
        }
    }
}

/// A scalar of our YAML subset: double-quoted (JSON-style escapes) or plain.
fn scalar(raw: &str) -> String {
    let raw = raw.trim();
    if raw.starts_with('"') {
        if let Ok(text) = serde_json::from_str::<String>(raw) {
            return text;
        }
    }
    raw.trim_matches(['"', '\'']).to_owned()
}

fn list(raw: &str) -> Vec<String> {
    raw.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(scalar)
        .filter(|item| !item.is_empty())
        .collect()
}

pub fn parse(id: &str, text: &str) -> AppResult<AgentProfile> {
    let text = text.replace("\r\n", "\n");
    let rest = text
        .strip_prefix("---\n")
        .ok_or_else(|| AppError::InvalidInput(format!("{id}: missing frontmatter")))?;
    let (head, body) = rest
        .split_once("\n---")
        .ok_or_else(|| AppError::InvalidInput(format!("{id}: unterminated frontmatter")))?;
    let mut profile = AgentProfile {
        id: id.to_owned(),
        name: id.to_owned(),
        description: String::new(),
        provider: None,
        model: None,
        permission_mode: None,
        tools: None,
        character: None,
        instructions: body.trim_start_matches(['-', '\n']).trim().to_owned(),
    };
    for line in head.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        let text = || Some(scalar(value)).filter(|v| !v.is_empty());
        match key.trim() {
            "name" => profile.name = text().unwrap_or(profile.name),
            "description" => profile.description = text().unwrap_or_default(),
            "provider" => profile.provider = text(),
            "model" => profile.model = text(),
            "character" => profile.character = text(),
            "permissionMode" => {
                profile.permission_mode =
                    serde_json::from_value(serde_json::Value::String(scalar(value))).ok();
            }
            "tools" => {
                // Unknown names are dropped: a profile can only narrow the tool set.
                let tools = list(value)
                    .into_iter()
                    .filter(|t| TOOL_NAMES.contains(&t.as_str()))
                    .collect();
                profile.tools = Some(tools);
            }
            _ => {}
        }
    }
    Ok(profile)
}

pub fn list_profiles(root: &Path) -> Vec<AgentProfile> {
    let Ok(entries) = std::fs::read_dir(root.join(PROFILES_DIR)) else {
        return Vec::new();
    };
    let mut profiles: Vec<AgentProfile> = entries
        .flatten()
        .filter(|e| {
            e.metadata()
                .is_ok_and(|m| m.is_file() && m.len() <= MAX_SIZE)
        })
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let id = name.strip_suffix(".md")?.to_owned();
            let text = std::fs::read_to_string(e.path()).ok()?;
            parse(&id, &text).ok()
        })
        .take(MAX_PROFILES)
        .collect();
    profiles.sort_by_key(|p| p.name.to_lowercase());
    profiles
}

pub fn get_profile(root: &Path, id: &str) -> AppResult<AgentProfile> {
    list_profiles(root)
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| AppError::NotFound(format!("agent profile {id}")))
}
