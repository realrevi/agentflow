//! What the commands ask the tracker: dashboard totals, panel badges, budgets (with the hard
//! stop and "allow once"), prices, context windows and the CLI readers' status.

use std::collections::BTreeMap;

use serde::Serialize;

use super::allow::AllowOnceGrant;
use super::budget::{self, BudgetStatus, Period};
use super::cli::CliKind;
use super::events::UsageEvent;
use super::price_cache;
use super::pricing::{openrouter_price, Price, PriceEntry};
use super::record::Source;
use super::summary::{day_start, panel_totals, spent, summarize, Summary, Totals};
use super::time::{civil, date_label, day_index};
use super::tracker::{State, UsageTracker};
use crate::error::{AppError, AppResult};
use crate::providers::ModelInfo;
use crate::settings::PriceOverride;
use crate::util::{lock, now_ms};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicPrice {
    pub model: String,
    #[serde(flatten)]
    pub price: Price,
}

/// Every price the app knows, USD per 1M tokens.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceList {
    /// `pricing.json`, each with its source URL and date.
    pub shipped: Vec<PriceEntry>,
    /// From OpenRouter's models endpoint (since the model list was last loaded).
    pub dynamic: Vec<DynamicPrice>,
    pub overrides: Vec<PriceOverride>,
    pub free_providers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliSupport {
    /// `claude-code`, `codex` or `gemini-cli`.
    pub source: Source,
    /// A documented, reliable usage source exists for this CLI (all three today).
    pub available: bool,
    /// The folder its session logs live in.
    pub root: Option<String>,
    /// The folder exists (the CLI has been used on this machine).
    pub found: bool,
    /// Reading is on and the folder is watched.
    pub watching: bool,
}

impl UsageTracker {
    pub fn summary(&self, offset_minutes: i32) -> Summary {
        let mut state = lock(&self.inner.state);
        state.offset = offset_minutes;
        summarize(&state.agg, &state.book, now_ms(), offset_minutes)
    }

    /// One panel's totals since `since_ms` (its session start).
    pub fn panel_totals(&self, panel: &str, since_ms: u64) -> Totals {
        let state = lock(&self.inner.state);
        panel_totals(&state.agg, &state.book, panel, since_ms)
    }

    pub fn budgets(&self, offset_minutes: i32) -> Vec<BudgetStatus> {
        lock(&self.inner.state).offset = offset_minutes;
        self.budget_statuses(now_ms())
    }

    pub(super) fn budget_statuses(&self, now: u64) -> Vec<BudgetStatus> {
        let state = lock(&self.inner.state);
        statuses(&state, now)
    }

    /// The current local day and month, which budget warnings are remembered per.
    pub(super) fn period_keys(&self, now: u64) -> (i64, String) {
        let offset = lock(&self.inner.state).offset;
        let day = day_index(now, offset);
        (day, date_label(day)[..7].to_owned())
    }

    /// Sends a `Budget` event for every budget that crossed 80% or 100%.
    pub(super) fn check_budgets(&self, now: u64) {
        let (day, month) = self.period_keys(now);
        let crossed = {
            let mut state = lock(&self.inner.state);
            if state.usage.budgets.daily.is_none()
                && state.usage.budgets.monthly.is_none()
                && state.usage.budgets.providers.is_empty()
            {
                return;
            }
            let current = statuses(&state, now);
            state.notified.crossings(&current, day, &month)
        };
        for status in crossed {
            self.emit(UsageEvent::Budget { status });
        }
    }

    /// The hard stop: an API chat send to `provider_id` is refused while a budget covering
    /// it is used up, unless `allow_once` is a valid token (which is then used up).
    pub fn check_send(&self, provider_id: &str, allow_once: Option<&str>) -> AppResult<()> {
        let now = now_ms();
        let mut state = lock(&self.inner.state);
        let current = statuses(&state, now);
        let Some(blocking) = budget::blocking(&state.usage.budgets, &current, provider_id) else {
            return Ok(());
        };
        let message = format!(
            "{} budget for {} used up (${:.2} of ${:.2})",
            match blocking.period {
                Period::Daily => "daily",
                Period::Monthly => "monthly",
            },
            blocking.scope,
            blocking.spent,
            blocking.limit
        );
        match allow_once {
            Some(token) if state.allow.consume(token, provider_id, now) => Ok(()),
            _ => Err(AppError::Budget(message)),
        }
    }

    pub fn allow_once(&self, provider_id: &str) -> AllowOnceGrant {
        lock(&self.inner.state).allow.grant(provider_id, now_ms())
    }

    pub fn price_for(&self, provider_id: &str, model: &str) -> Option<Price> {
        lock(&self.inner.state).book.price_for(provider_id, model)
    }

    pub fn prices(&self) -> PriceList {
        let state = lock(&self.inner.state);
        let mut dynamic: Vec<DynamicPrice> = state
            .book
            .dynamic()
            .map(|(model, price)| DynamicPrice {
                model: model.clone(),
                price: *price,
            })
            .collect();
        dynamic.sort_by(|a, b| a.model.cmp(&b.model));
        PriceList {
            shipped: state.book.entries().to_vec(),
            dynamic,
            overrides: state.usage.prices.clone(),
            free_providers: state.usage.free_providers.clone(),
        }
    }

    /// A provider's model list: context windows, and OpenRouter's per-token prices.
    pub fn note_models(&self, provider_id: &str, preset: &str, models: &[ModelInfo]) {
        let mut repriced = false;
        let cached = {
            let mut state = lock(&self.inner.state);
            for model in models {
                if let Some(window) = model.context_window {
                    state
                        .windows
                        .insert(format!("{provider_id}|{}", model.id), window);
                }
                let price = (preset == "openrouter")
                    .then(|| model.pricing.as_ref().and_then(openrouter_price))
                    .flatten();
                if let Some(price) = price {
                    repriced |= state.book.set_dynamic(&model.id, price);
                }
            }
            let all: BTreeMap<String, Price> = state
                .book
                .dynamic()
                .map(|(model, price)| (model.clone(), *price))
                .collect();
            all
        };
        if repriced {
            price_cache::save(&self.inner.dir.join(price_cache::FILE), &cached);
            self.emit(UsageEvent::Repriced);
        }
    }

    /// The context window a models endpoint reported, if any.
    pub fn context_window(&self, provider_id: &str, model: &str) -> Option<u64> {
        lock(&self.inner.state)
            .windows
            .get(&format!("{provider_id}|{model}"))
            .copied()
    }

    pub fn cli_status(&self) -> Vec<CliSupport> {
        let cli = lock(&self.inner.cli);
        let tracking = cli.watch.is_some();
        CliKind::ALL
            .into_iter()
            .map(|kind| {
                let root = cli.roots.root(kind);
                let found = root.is_some_and(|r| r.is_dir());
                CliSupport {
                    source: kind.source(),
                    available: true,
                    root: root.map(|r| r.to_string_lossy().into_owned()),
                    found,
                    watching: tracking && found,
                }
            })
            .collect()
    }
}

fn statuses(state: &State, now: u64) -> Vec<BudgetStatus> {
    let today = day_index(now, state.offset);
    let (_, _, day_of_month) = civil(today);
    let day_from = day_start(today, state.offset);
    let month_from = day_start(today - (i64::from(day_of_month) - 1), state.offset);
    budget::statuses(&state.usage.budgets, |scope, period| {
        let from = match period {
            Period::Daily => day_from,
            Period::Monthly => month_from,
        };
        spent(&state.agg, &state.book, from, scope)
    })
}
