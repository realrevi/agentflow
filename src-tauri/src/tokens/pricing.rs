//! Prices per 1M tokens: shipped `pricing.json` (official prices, each with a source URL and
//! date), OpenRouter prices from its models endpoint, and the user's overrides. A model
//! without a price has no cost ("—"): a price is never guessed.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::providers::TokenUsage;
use crate::settings::UsageSettings;

const SHIPPED: &str = include_str!("pricing.json");

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

impl Price {
    pub const FREE: Price = Price {
        input: 0.0,
        output: 0.0,
        cache_read: 0.0,
        cache_write: 0.0,
    };

    /// USD for `tokens`.
    pub fn cost(&self, tokens: &TokenUsage) -> f64 {
        let per = |count: u64, price: f64| count as f64 * price / 1_000_000.0;
        per(tokens.input_tokens, self.input)
            + per(tokens.output_tokens, self.output)
            + per(tokens.cache_read_tokens, self.cache_read)
            + per(tokens.cache_write_tokens, self.cache_write)
    }

    /// What the cache reads saved compared with paying the full input price for them.
    pub fn cache_savings(&self, tokens: &TokenUsage) -> f64 {
        tokens.cache_read_tokens as f64 * (self.input - self.cache_read).max(0.0) / 1_000_000.0
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceEntry {
    pub model: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub source: String,
    pub updated: String,
    #[serde(default)]
    pub note: Option<String>,
}

impl PriceEntry {
    fn price(&self) -> Price {
        Price {
            input: self.input,
            output: self.output,
            cache_read: self.cache_read,
            cache_write: self.cache_write,
        }
    }
}

#[derive(Deserialize)]
struct Shipped {
    models: Vec<PriceEntry>,
}

/// Everything needed to price a record. Rebuilt when settings change.
#[derive(Debug, Clone, Default)]
pub struct PriceBook {
    shipped: HashMap<String, Price>,
    entries: Vec<PriceEntry>,
    overrides: HashMap<String, Price>,
    /// Prices reported by a provider's models endpoint (OpenRouter), by model id.
    dynamic: HashMap<String, Price>,
    free_providers: Vec<String>,
}

impl PriceBook {
    pub fn shipped() -> Self {
        let entries = match serde_json::from_str::<Shipped>(SHIPPED) {
            Ok(shipped) => shipped.models,
            Err(err) => {
                tracing::error!("pricing.json is invalid: {err}");
                Vec::new()
            }
        };
        let mut shipped = HashMap::new();
        for entry in &entries {
            shipped.insert(entry.model.clone(), entry.price());
            for alias in &entry.aliases {
                shipped.insert(alias.clone(), entry.price());
            }
        }
        Self {
            shipped,
            entries,
            ..Self::default()
        }
    }

    pub fn entries(&self) -> &[PriceEntry] {
        &self.entries
    }

    pub fn apply_settings(&mut self, usage: &UsageSettings) {
        self.overrides = usage
            .prices
            .iter()
            .map(|p| {
                let price = Price {
                    input: p.input,
                    output: p.output,
                    cache_read: p.cache_read,
                    cache_write: p.cache_write,
                };
                (p.model.clone(), price)
            })
            .collect();
        self.free_providers = usage.free_providers.clone();
    }

    /// Returns whether the price is new or different.
    pub fn set_dynamic(&mut self, model: &str, price: Price) -> bool {
        self.dynamic.insert(model.to_owned(), price) != Some(price)
    }

    pub fn dynamic(&self) -> impl Iterator<Item = (&String, &Price)> {
        self.dynamic.iter()
    }

    /// The price for a model used through `provider` (an entry id or a CLI key).
    /// Free providers cost nothing; the user's prices win over shipped ones.
    pub fn price_for(&self, provider: &str, model: &str) -> Option<Price> {
        if self.free_providers.iter().any(|p| p == provider) {
            return Some(Price::FREE);
        }
        if let Some(price) = self.overrides.get(model) {
            return Some(*price);
        }
        if let Some(price) = self.dynamic.get(model) {
            return Some(*price);
        }
        candidates(model).find_map(|name| {
            self.overrides
                .get(&name)
                .or_else(|| self.shipped.get(&name))
                .copied()
        })
    }
}

/// Names to try for a model id: as is, without a vendor prefix (`anthropic/…`, `models/…`),
/// without a date snapshot suffix (`-20250514`), and with dots as dashes (`claude-opus-4.8`).
fn candidates(model: &str) -> impl Iterator<Item = String> {
    let bare = model.rsplit('/').next().unwrap_or(model).trim().to_owned();
    let undated = strip_date(&bare);
    let dashed = undated.replace('.', "-");
    let mut out = vec![model.to_owned(), bare, undated, dashed];
    out.dedup();
    out.into_iter()
}

fn strip_date(model: &str) -> String {
    match model.rsplit_once('-') {
        Some((head, tail)) if tail.len() == 8 && tail.chars().all(|c| c.is_ascii_digit()) => {
            head.to_owned()
        }
        _ => model.to_owned(),
    }
}

/// OpenRouter `pricing` values are USD per token, as strings.
pub fn openrouter_price(pricing: &serde_json::Value) -> Option<Price> {
    let per_million = |key: &str| -> Option<f64> {
        let value = &pricing[key];
        let number = value
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| value.as_f64())?;
        (number.is_finite() && number >= 0.0).then_some(number * 1_000_000.0)
    };
    let input = per_million("prompt")?;
    let output = per_million("completion")?;
    Some(Price {
        input,
        output,
        cache_read: per_million("input_cache_read").unwrap_or(input),
        cache_write: per_million("input_cache_write").unwrap_or(input),
    })
}

#[cfg(test)]
#[path = "pricing_tests.rs"]
mod tests;
