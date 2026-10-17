//! Dashboard queries over the aggregator: totals (today / 7d / 30d), a daily series and
//! breakdowns by provider/model, workspace and panel; panel session totals; budget spend.

use std::collections::HashMap;

use serde::Serialize;

use super::aggregate::{Aggregator, DAY_MS};
use super::pricing::PriceBook;
use super::record::Source;
use super::time::{date_label, day_index};
pub use super::totals::Totals;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    /// `YYYY-MM-DD` (local).
    pub date: String,
    #[serde(flatten)]
    pub totals: Totals,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    /// Model id, workspace path or panel id.
    pub key: String,
    pub source: Source,
    /// Provider entry id or CLI key.
    pub provider: String,
    #[serde(flatten)]
    pub totals: Totals,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub today: Totals,
    pub week: Totals,
    pub month: Totals,
    /// The last 30 local days, oldest first.
    pub days: Vec<Day>,
    /// Last 30 days, most expensive first.
    pub models: Vec<Group>,
    pub workspaces: Vec<Group>,
    pub panels: Vec<Group>,
    /// Compact summaries (30 days).
    pub compact: Totals,
}

pub const DAYS: i64 = 30;
const MAX_GROUPS: usize = 50;

type GroupKey = (Source, String, String);

/// Local midnight (ms) of a day index.
pub fn day_start(day: i64, offset_minutes: i32) -> u64 {
    let ms = day * DAY_MS as i64 - i64::from(offset_minutes) * 60_000;
    u64::try_from(ms).unwrap_or(0)
}

pub fn summarize(agg: &Aggregator, book: &PriceBook, now_ms: u64, offset: i32) -> Summary {
    let today = day_index(now_ms, offset);
    let first = today - (DAYS - 1);
    let mut days: Vec<Totals> = vec![Totals::default(); DAYS as usize];
    let mut models: HashMap<GroupKey, Totals> = HashMap::new();
    let mut workspaces: HashMap<GroupKey, Totals> = HashMap::new();
    let mut panels: HashMap<GroupKey, Totals> = HashMap::new();
    let mut compact = Totals::default();
    agg.visit(day_start(first, offset), now_ms + 1, |at, key, counts| {
        let index = (day_index(at, offset) - first).clamp(0, DAYS - 1) as usize;
        days[index].add(key, counts, book);
        let group = |map: &mut HashMap<GroupKey, Totals>, name: &str| {
            map.entry((key.source, key.provider.to_string(), name.to_owned()))
                .or_default()
                .add(key, counts, book);
        };
        group(&mut models, &key.model);
        group(&mut workspaces, key.workspace.as_deref().unwrap_or(""));
        if let Some(panel) = &key.panel {
            group(&mut panels, panel);
        }
        if key.compact {
            compact.add(key, counts, book);
        }
    });
    let sum = |range: std::ops::Range<usize>| {
        let mut total = Totals::default();
        for day in &days[range] {
            total.merge(day);
        }
        total
    };
    let n = DAYS as usize;
    Summary {
        today: days[n - 1],
        week: sum(n - 7..n),
        month: sum(0..n),
        days: days
            .iter()
            .enumerate()
            .map(|(i, totals)| Day {
                date: date_label(first + i as i64),
                totals: *totals,
            })
            .collect(),
        models: sorted(models),
        workspaces: sorted(workspaces),
        panels: sorted(panels),
        compact,
    }
}

/// Session totals of one panel since `since_ms` (the corner badge).
pub fn panel_totals(agg: &Aggregator, book: &PriceBook, panel: &str, since_ms: u64) -> Totals {
    let mut total = Totals::default();
    agg.visit(since_ms, u64::MAX, |_, key, counts| {
        if key.panel.as_deref() == Some(panel) {
            total.add(key, counts, book);
        }
    });
    total
}

/// USD spent from `from_ms` on, for budgets: all sources, or one provider / CLI key.
pub fn spent(agg: &Aggregator, book: &PriceBook, from_ms: u64, scope: Option<&str>) -> f64 {
    let mut cost = 0.0;
    agg.visit(from_ms, u64::MAX, |_, key, counts| {
        if scope.is_none_or(|s| *key.provider == *s) {
            if let Some(price) = book.price_for(&key.provider, &key.model) {
                cost += price.cost(&counts.tokens);
            }
        }
    });
    cost
}

fn sorted(map: HashMap<GroupKey, Totals>) -> Vec<Group> {
    let mut groups: Vec<Group> = map
        .into_iter()
        .map(|((source, provider, key), totals)| Group {
            key,
            source,
            provider,
            totals,
        })
        .collect();
    groups.sort_by(|a, b| {
        b.totals
            .cost
            .total_cmp(&a.totals.cost)
            .then(
                b.totals
                    .tokens
                    .output_tokens
                    .cmp(&a.totals.tokens.output_tokens),
            )
            .then(a.key.cmp(&b.key))
    });
    groups.truncate(MAX_GROUPS);
    groups
}

#[cfg(test)]
#[path = "summary_tests.rs"]
mod tests;
