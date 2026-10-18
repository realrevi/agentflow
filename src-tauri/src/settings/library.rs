//! The community agent library (phase 10): the user's own (unverified) GitHub sources and
//! the daily automatic refresh. The optional GitHub token lives in the OS keychain.

use serde::{Deserialize, Serialize};

pub const MAX_CUSTOM_SOURCES: usize = 20;
const MAX_URL_LEN: usize = 300;
const MAX_PATH_LEN: usize = 200;

/// A public GitHub repository the user added (`https://github.com/owner/name`, optionally
/// `/tree/<branch>/<folder>`), plus an optional folder inside it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CustomSource {
    pub url: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LibrarySettings {
    pub custom_sources: Vec<CustomSource>,
    /// Refresh the library when it is opened and the last refresh is over a day old.
    pub auto_refresh: bool,
}

impl Default for LibrarySettings {
    fn default() -> Self {
        Self {
            custom_sources: Vec::new(),
            auto_refresh: true,
        }
    }
}

impl LibrarySettings {
    pub fn normalized(mut self) -> Self {
        let mut seen = Vec::new();
        self.custom_sources.retain_mut(|source| {
            source.url = source.url.trim().trim_end_matches('/').to_owned();
            source.path = source.path.trim().trim_matches('/').to_owned();
            let key = (source.url.to_ascii_lowercase(), source.path.clone());
            let ok = source.url.starts_with("https://github.com/")
                && source.url.len() <= MAX_URL_LEN
                && source.path.len() <= MAX_PATH_LEN
                && !seen.contains(&key);
            if ok {
                seen.push(key);
            }
            ok
        });
        self.custom_sources.truncate(MAX_CUSTOM_SOURCES);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_github_urls_once_and_trims_them() {
        let source = |url: &str, path: &str| CustomSource {
            url: url.into(),
            path: path.into(),
        };
        let settings = LibrarySettings {
            custom_sources: vec![
                source(" https://github.com/acme/agents/ ", "/agents/"),
                source("https://github.com/acme/agents", "agents"),
                source("http://github.com/acme/agents", ""),
                source("https://example.com/acme/agents", ""),
            ],
            auto_refresh: false,
        }
        .normalized();
        assert_eq!(
            settings.custom_sources,
            vec![source("https://github.com/acme/agents", "agents")]
        );
        assert!(!settings.auto_refresh);
    }
}
