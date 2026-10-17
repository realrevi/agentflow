//! Budgets: daily and monthly limits, globally and per provider, with warnings at 80% and
//! 100%. The optional hard stop blocks API chat sends only; CLI agents can only be warned.

use std::collections::HashMap;

use serde::Serialize;

use crate::settings::Budgets;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Period {
    Daily,
    Monthly,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetStatus {
    /// `all`, a provider entry id or a CLI key (`claude-code`, `codex`, `gemini-cli`).
    pub scope: String,
    pub period: Period,
    pub limit: f64,
    pub spent: f64,
    /// 0, 80 or 100.
    pub level: u8,
}

pub const ALL: &str = "all";

pub fn level(spent: f64, limit: f64) -> u8 {
    if limit <= 0.0 {
        0
    } else if spent >= limit {
        100
    } else if spent >= limit * 0.8 {
        80
    } else {
        0
    }
}

/// Every configured budget with what was spent in its period. `spent(scope, period)` gives
/// the USD spent; `None` scope = all sources.
pub fn statuses(
    budgets: &Budgets,
    spent: impl Fn(Option<&str>, Period) -> f64,
) -> Vec<BudgetStatus> {
    let mut out = Vec::new();
    let mut push = |scope: &str, period: Period, limit: Option<f64>| {
        if let Some(limit) = limit {
            let used = spent((scope != ALL).then_some(scope), period);
            out.push(BudgetStatus {
                scope: scope.to_owned(),
                period,
                limit,
                spent: used,
                level: level(used, limit),
            });
        }
    };
    push(ALL, Period::Daily, budgets.daily);
    push(ALL, Period::Monthly, budgets.monthly);
    for entry in &budgets.providers {
        push(&entry.provider, Period::Daily, entry.daily);
        push(&entry.provider, Period::Monthly, entry.monthly);
    }
    out
}

/// Remembers the level each budget was last reported at, per period instance (day/month).
#[derive(Debug, Default)]
pub struct Notified {
    levels: HashMap<String, u8>,
}

impl Notified {
    /// The statuses whose level went up since the last call. `day` and `month` identify the
    /// current periods, so a new day starts from zero again.
    pub fn crossings(
        &mut self,
        statuses: &[BudgetStatus],
        day: i64,
        month: &str,
    ) -> Vec<BudgetStatus> {
        let mut out = Vec::new();
        for status in statuses {
            let instance = match status.period {
                Period::Daily => day.to_string(),
                Period::Monthly => month.to_owned(),
            };
            let key = format!("{}|{:?}|{instance}", status.scope, status.period);
            let before = self.levels.get(&key).copied().unwrap_or(0);
            if status.level > before {
                out.push(status.clone());
            }
            self.levels.insert(key, status.level.max(before));
        }
        out
    }

    /// Records the current levels without reporting them (app start: no toast storm).
    pub fn baseline(&mut self, statuses: &[BudgetStatus], day: i64, month: &str) {
        let _ = self.crossings(statuses, day, month);
    }
}

/// The budget that blocks an API chat send to `provider_id`, if the hard stop is on and a
/// budget covering it (global or that provider) is used up.
pub fn blocking<'a>(
    budgets: &Budgets,
    statuses: &'a [BudgetStatus],
    provider_id: &str,
) -> Option<&'a BudgetStatus> {
    if !budgets.hard_stop {
        return None;
    }
    statuses
        .iter()
        .find(|s| s.level >= 100 && (s.scope == ALL || s.scope == provider_id))
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
