//! In-memory usage totals: token counts per 15-minute bucket and key (source, provider,
//! model, workspace, panel), built from the ledger at startup and updated as records arrive.
//! Costs are computed at query time (cost is linear in tokens), so a price edit needs no
//! rebuild. Every UTC offset is a multiple of 15 minutes, so local days stay exact.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::record::{Source, UsageRecord};
use crate::providers::TokenUsage;

pub const QUARTER_MS: u64 = 900_000;
pub const DAY_MS: u64 = 86_400_000;
/// Records older than this only count for dedup (queries look back at most ~31 days).
pub const WINDOW_DAYS: u64 = 62;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    pub source: Source,
    pub provider: Arc<str>,
    pub model: Arc<str>,
    pub workspace: Option<Arc<str>>,
    pub panel: Option<Arc<str>>,
    /// Summaries written by Compact.
    pub compact: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub tokens: TokenUsage,
    pub requests: u64,
}

impl Counts {
    pub fn add(&mut self, tokens: &TokenUsage, requests: u64) {
        let t = &mut self.tokens;
        t.input_tokens += tokens.input_tokens;
        t.output_tokens += tokens.output_tokens;
        t.cache_read_tokens += tokens.cache_read_tokens;
        t.cache_write_tokens += tokens.cache_write_tokens;
        self.requests += requests;
    }
}

#[derive(Debug, Default)]
pub struct Aggregator {
    buckets: BTreeMap<u64, HashMap<Key, Counts>>,
    /// Hashes of `source|id` of every record with an id (dedup across files and restarts).
    seen: HashSet<u64>,
    strings: HashSet<Arc<str>>,
}

impl Aggregator {
    /// Whether `record` would be new (no id, or an id not seen yet).
    pub fn is_new(&self, record: &UsageRecord) -> bool {
        record.id.is_empty() || !self.seen.contains(&id_hash(record.source, &record.id))
    }

    /// Adds a record unless its id was seen; returns whether it was new.
    pub fn add(&mut self, record: &UsageRecord, now_ms: u64) -> bool {
        if !record.id.is_empty() && !self.seen.insert(id_hash(record.source, &record.id)) {
            return false;
        }
        if record.at.saturating_add(WINDOW_DAYS * DAY_MS) < now_ms {
            return true; // new, but too old to matter for any total
        }
        let key = Key {
            source: record.source,
            provider: self.intern(&record.provider),
            model: self.intern(&record.model),
            workspace: record.workspace.as_deref().map(|w| self.intern(w)),
            panel: record.panel.as_deref().map(|p| self.intern(p)),
            compact: record.kind == "compact",
        };
        self.buckets
            .entry(record.at / QUARTER_MS)
            .or_default()
            .entry(key)
            .or_default()
            .add(&record.tokens, 1);
        true
    }

    /// Calls `visit(bucket start ms, key, counts)` for every bucket in `[from_ms, to_ms)`.
    pub fn visit(&self, from_ms: u64, to_ms: u64, mut visit: impl FnMut(u64, &Key, &Counts)) {
        let range = (from_ms / QUARTER_MS)..to_ms.div_ceil(QUARTER_MS);
        for (quarter, keys) in self.buckets.range(range) {
            let start = quarter * QUARTER_MS;
            if start < from_ms || start >= to_ms {
                continue;
            }
            for (key, counts) in keys {
                visit(start, key, counts);
            }
        }
    }

    /// Drops buckets that fell out of the window (ids stay for dedup).
    pub fn prune(&mut self, now_ms: u64) {
        let keep_from = now_ms.saturating_sub(WINDOW_DAYS * DAY_MS) / QUARTER_MS;
        self.buckets = self.buckets.split_off(&keep_from);
    }

    fn intern(&mut self, text: &str) -> Arc<str> {
        if let Some(existing) = self.strings.get(text) {
            return Arc::clone(existing);
        }
        let new: Arc<str> = Arc::from(text);
        self.strings.insert(Arc::clone(&new));
        new
    }
}

fn id_hash(source: Source, id: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    id.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
#[path = "aggregate_tests.rs"]
mod tests;
