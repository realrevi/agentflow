//! OpenRouter prices from its models endpoint, kept in `<app data>/usage/openrouter-prices.json`
//! so their costs are known right after a restart, before the model list loads again.

use std::collections::BTreeMap;
use std::path::Path;

use super::pricing::Price;
use crate::util::write_atomic;

pub const FILE: &str = "openrouter-prices.json";

/// A missing or unreadable file is an empty cache.
pub fn load(path: &Path) -> BTreeMap<String, Price> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, prices: &BTreeMap<String, Price>) {
    let written = serde_json::to_vec(prices)
        .map_err(|err| err.to_string())
        .and_then(|bytes| write_atomic(path, &bytes).map_err(|err| err.to_string()));
    if let Err(err) = written {
        tracing::warn!("cannot save OpenRouter prices: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_dir::TestDir;

    #[test]
    fn round_trips_and_tolerates_a_bad_file() {
        let dir = TestDir::new("price-cache");
        let path = dir.path().join(FILE);
        assert!(load(&path).is_empty());
        let mut prices = BTreeMap::new();
        prices.insert(
            "openai/gpt-5".to_owned(),
            Price {
                input: 1.25,
                output: 10.0,
                cache_read: 0.125,
                cache_write: 1.25,
            },
        );
        save(&path, &prices);
        assert_eq!(load(&path), prices);
        std::fs::write(&path, b"{oops").expect("write");
        assert!(load(&path).is_empty());
    }
}
