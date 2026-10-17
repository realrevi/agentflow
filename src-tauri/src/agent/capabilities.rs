//! Which models support tools. Default: yes. The user's settings override; a provider that
//! rejects tools for a model is remembered here (`<app data>/model-tools.json`, written by
//! the backend only, so frontend settings saves never overwrite it).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::providers::{ErrorKind, ProviderError};
use crate::settings::AgentSettings;
use crate::util::{lock, write_atomic};

#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    path: PathBuf,
    /// `provider|model` -> supports tools.
    learned: Arc<Mutex<BTreeMap<String, bool>>>,
}

impl ModelCapabilities {
    pub fn load(path: PathBuf) -> Self {
        let learned = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            path,
            learned: Arc::new(Mutex::new(learned)),
        }
    }

    pub fn supports_tools(&self, settings: &AgentSettings, provider: &str, model: &str) -> bool {
        settings
            .tools_override(model)
            .or_else(|| lock(&self.learned).get(&key(provider, model)).copied())
            .unwrap_or(true)
    }

    /// The provider rejected tools for this model: remember it.
    pub fn remember_unsupported(&self, provider: &str, model: &str) {
        let snapshot = {
            let mut learned = lock(&self.learned);
            learned.insert(key(provider, model), false);
            learned.clone()
        };
        match serde_json::to_vec_pretty(&snapshot) {
            Ok(bytes) => {
                if let Err(err) = write_atomic(&self.path, &bytes) {
                    tracing::warn!("cannot save model capabilities: {err}");
                }
            }
            Err(err) => tracing::warn!("cannot encode model capabilities: {err}"),
        }
    }
}

fn key(provider: &str, model: &str) -> String {
    format!("{provider}|{model}")
}

/// A 400-type error that is about tools ("does not support tools", "tool use is not
/// supported", "function calling is not enabled" ...).
pub fn rejects_tools(err: &ProviderError) -> bool {
    let about_tools = {
        let message = err.message.to_ascii_lowercase();
        ["tool", "function"]
            .iter()
            .any(|word| message.contains(word))
    };
    about_tools && matches!(err.kind, ErrorKind::BadRequest | ErrorKind::InvalidModel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ModelTools;
    use crate::util::test_dir::TestDir;

    #[test]
    fn overrides_win_and_learned_flags_persist() {
        let dir = TestDir::new("capabilities");
        let path = dir.path().join("model-tools.json");
        let caps = ModelCapabilities::load(path.clone());
        let mut settings = AgentSettings::default();
        assert!(caps.supports_tools(&settings, "p1", "m1"));
        caps.remember_unsupported("p1", "m1");
        assert!(!caps.supports_tools(&settings, "p1", "m1"));
        assert!(caps.supports_tools(&settings, "p2", "m1"), "per provider");
        assert!(!ModelCapabilities::load(path).supports_tools(&settings, "p1", "m1"));
        settings.tool_models.push(ModelTools {
            model: "m1".into(),
            tools: true,
        });
        assert!(
            caps.supports_tools(&settings, "p1", "m1"),
            "the user's override wins"
        );
    }

    #[test]
    fn recognizes_tool_rejections() {
        let rejected = |kind, message: &str| rejects_tools(&ProviderError::new(kind, message));
        assert!(rejected(
            ErrorKind::BadRequest,
            "llama3 does not support tools"
        ));
        assert!(rejected(
            ErrorKind::BadRequest,
            "Function calling is not enabled for this model"
        ));
        assert!(!rejected(ErrorKind::BadRequest, "max_tokens is too large"));
        assert!(!rejected(ErrorKind::Auth, "invalid key for tool access"));
    }
}
