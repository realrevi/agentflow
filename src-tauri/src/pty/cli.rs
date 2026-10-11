//! Detects the agent CLIs (claude, codex, gemini) on PATH. Results are cached until a
//! refresh is requested, because the "+" menu asks often and PATH rarely changes.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;

use super::model::PtyKind;
use crate::util::lock;

const AGENT_CLIS: [(PtyKind, &str); 3] = [
    (PtyKind::Claude, "claude"),
    (PtyKind::Codex, "codex"),
    (PtyKind::Gemini, "gemini"),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    pub kind: PtyKind,
    pub binary: String,
    pub found: bool,
    pub path: Option<String>,
}

/// Cheap to clone: clones share one cache.
#[derive(Debug, Clone, Default)]
pub struct CliDetector {
    cache: Arc<Mutex<Option<Vec<CliStatus>>>>,
}

impl CliDetector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn detect(&self, refresh: bool) -> Vec<CliStatus> {
        self.detect_with(refresh, |binary| which::which(binary).ok())
    }

    fn detect_with(&self, refresh: bool, find: impl Fn(&str) -> Option<PathBuf>) -> Vec<CliStatus> {
        let mut cache = lock(&self.cache);
        if let (false, Some(cached)) = (refresh, cache.as_ref()) {
            return cached.clone();
        }
        let fresh: Vec<CliStatus> = AGENT_CLIS
            .iter()
            .map(|&(kind, binary)| {
                let path = find(binary);
                CliStatus {
                    kind,
                    binary: binary.into(),
                    found: path.is_some(),
                    path: path.map(|p| p.to_string_lossy().into_owned()),
                }
            })
            .collect();
        *cache = Some(fresh.clone());
        fresh
    }
}

/// The executable name for an agent kind, or `None` for shell/custom.
pub fn agent_binary(kind: PtyKind) -> Option<&'static str> {
    AGENT_CLIS
        .iter()
        .find(|(candidate, _)| *candidate == kind)
        .map(|&(_, binary)| binary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn caches_until_refresh() {
        let detector = CliDetector::new();
        let calls = AtomicUsize::new(0);
        let find = |binary: &str| {
            calls.fetch_add(1, Ordering::Relaxed);
            (binary == "claude").then(|| PathBuf::from("/usr/local/bin/claude"))
        };

        let first = detector.detect_with(false, find);
        assert_eq!(calls.load(Ordering::Relaxed), 3);
        assert!(first[0].found);
        assert_eq!(first[0].path.as_deref(), Some("/usr/local/bin/claude"));
        assert!(!first[1].found && first[1].path.is_none());

        let cached = detector.detect_with(false, find);
        assert_eq!(cached, first);
        assert_eq!(calls.load(Ordering::Relaxed), 3);

        detector.detect_with(true, find);
        assert_eq!(calls.load(Ordering::Relaxed), 6);
    }

    #[test]
    fn maps_agent_kinds_to_binaries() {
        assert_eq!(agent_binary(PtyKind::Codex), Some("codex"));
        assert_eq!(agent_binary(PtyKind::Shell), None);
        assert_eq!(agent_binary(PtyKind::Custom), None);
    }
}
