//! Token & cost tracking options (phase 6): CLI readers, subscription label, price overrides,
//! free providers and budgets. Prices are USD per 1M tokens.

use serde::{Deserialize, Serialize};

use super::chat::clip;

pub const MAX_PRICE: f64 = 10_000.0;
pub const MAX_BUDGET: f64 = 1_000_000.0;
const MAX_OVERRIDES: usize = 200;
const MAX_KEY_LEN: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UsageSettings {
    /// Read Claude Code / Codex / Gemini CLI session logs (read-only).
    pub track_cli: bool,
    /// "I use a subscription for Claude Code / Codex": CLI cost is labeled API-equivalent.
    pub cli_subscription: bool,
    /// Provider entry ids whose usage costs $0 (e.g. free credits).
    pub free_providers: Vec<String>,
    /// User prices: replace a shipped price or add a custom model.
    pub prices: Vec<PriceOverride>,
    pub budgets: Budgets,
}

impl Default for UsageSettings {
    fn default() -> Self {
        Self {
            track_cli: true,
            cli_subscription: false,
            free_providers: Vec::new(),
            prices: Vec::new(),
            budgets: Budgets::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PriceOverride {
    pub model: String,
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Budgets {
    /// USD per local day, all sources.
    pub daily: Option<f64>,
    /// USD per calendar month, all sources.
    pub monthly: Option<f64>,
    pub providers: Vec<ProviderBudget>,
    /// Block API chat sends while over budget (CLI agents can only be warned).
    pub hard_stop: bool,
}

/// `provider` is a provider entry id (API) or `claude-code` / `codex` / `gemini-cli`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProviderBudget {
    pub provider: String,
    pub daily: Option<f64>,
    pub monthly: Option<f64>,
}

impl UsageSettings {
    pub fn normalized(mut self) -> Self {
        let mut free: Vec<String> = Vec::new();
        for id in std::mem::take(&mut self.free_providers) {
            let id = clip(id.trim(), MAX_KEY_LEN);
            if !id.is_empty() && !free.contains(&id) {
                free.push(id);
            }
        }
        self.free_providers = free;
        let mut prices: Vec<PriceOverride> = Vec::new();
        for mut price in std::mem::take(&mut self.prices) {
            price.model = clip(price.model.trim(), MAX_KEY_LEN);
            if price.model.is_empty() || prices.iter().any(|p| p.model == price.model) {
                continue;
            }
            for value in [
                &mut price.input,
                &mut price.output,
                &mut price.cache_read,
                &mut price.cache_write,
            ] {
                *value = money(*value, MAX_PRICE).unwrap_or(0.0);
            }
            prices.push(price);
        }
        prices.truncate(MAX_OVERRIDES);
        self.prices = prices;
        self.budgets = self.budgets.normalized();
        self
    }
}

impl Budgets {
    fn normalized(mut self) -> Self {
        self.daily = self.daily.and_then(budget);
        self.monthly = self.monthly.and_then(budget);
        let mut providers: Vec<ProviderBudget> = Vec::new();
        for mut entry in std::mem::take(&mut self.providers) {
            entry.provider = clip(entry.provider.trim(), MAX_KEY_LEN);
            entry.daily = entry.daily.and_then(budget);
            entry.monthly = entry.monthly.and_then(budget);
            let empty = entry.daily.is_none() && entry.monthly.is_none();
            if entry.provider.is_empty()
                || empty
                || providers.iter().any(|p| p.provider == entry.provider)
            {
                continue;
            }
            providers.push(entry);
        }
        providers.truncate(MAX_OVERRIDES);
        self.providers = providers;
        self
    }
}

/// A finite, non-negative amount, capped.
fn money(value: f64, max: f64) -> Option<f64> {
    (value.is_finite() && value >= 0.0).then(|| value.min(max))
}

/// Budgets must be positive to mean anything.
fn budget(value: f64) -> Option<f64> {
    money(value, MAX_BUDGET).filter(|v| *v > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_prices_and_budgets() {
        let usage = UsageSettings {
            free_providers: vec![" p1 ".into(), "p1".into(), String::new()],
            prices: vec![
                PriceOverride {
                    model: " my-model ".into(),
                    input: -1.0,
                    output: f64::INFINITY,
                    cache_read: 0.5,
                    cache_write: 1e9,
                },
                PriceOverride {
                    model: "my-model".into(),
                    ..PriceOverride::default()
                },
            ],
            budgets: Budgets {
                daily: Some(0.0),
                monthly: Some(50.0),
                providers: vec![
                    ProviderBudget {
                        provider: "codex".into(),
                        daily: Some(2.0),
                        monthly: None,
                    },
                    ProviderBudget {
                        provider: "empty".into(),
                        daily: Some(-3.0),
                        monthly: None,
                    },
                ],
                hard_stop: true,
            },
            ..UsageSettings::default()
        }
        .normalized();
        assert_eq!(usage.free_providers, vec!["p1".to_owned()]);
        assert_eq!(usage.prices.len(), 1);
        let price = &usage.prices[0];
        assert_eq!(price.model, "my-model");
        assert_eq!((price.input, price.output), (0.0, 0.0));
        assert_eq!((price.cache_read, price.cache_write), (0.5, MAX_PRICE));
        assert_eq!(usage.budgets.daily, None);
        assert_eq!(usage.budgets.monthly, Some(50.0));
        assert_eq!(usage.budgets.providers.len(), 1);
        assert!(usage.budgets.hard_stop);
    }
}
