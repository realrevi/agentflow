//! Summed tokens with their cost: the priced part, unpriced tokens (cost unknown, shown as
//! "—"), cache savings and the CLI share of the cost.

use serde::Serialize;

use super::aggregate::{Counts, Key};
use super::pricing::PriceBook;
use crate::providers::TokenUsage;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    #[serde(flatten)]
    pub tokens: TokenUsage,
    pub requests: u64,
    /// USD of the priced part.
    pub cost: f64,
    /// Tokens of records without a price (their cost is unknown).
    pub unpriced_tokens: u64,
    /// What cache reads saved vs. the full input price.
    pub cache_savings: f64,
    /// USD of CLI usage (labelled "API-equivalent" with a subscription).
    pub cli_cost: f64,
}

impl Totals {
    pub fn add(&mut self, key: &Key, counts: &Counts, book: &PriceBook) {
        let c = &counts.tokens;
        add_tokens(&mut self.tokens, c);
        self.requests += counts.requests;
        match book.price_for(&key.provider, &key.model) {
            Some(price) => {
                let cost = price.cost(c);
                self.cost += cost;
                self.cache_savings += price.cache_savings(c);
                if key.source.is_cli() {
                    self.cli_cost += cost;
                }
            }
            None => {
                self.unpriced_tokens +=
                    c.input_tokens + c.output_tokens + c.cache_read_tokens + c.cache_write_tokens
            }
        }
    }

    pub fn merge(&mut self, other: &Totals) {
        add_tokens(&mut self.tokens, &other.tokens);
        self.requests += other.requests;
        self.cost += other.cost;
        self.unpriced_tokens += other.unpriced_tokens;
        self.cache_savings += other.cache_savings;
        self.cli_cost += other.cli_cost;
    }
}

fn add_tokens(into: &mut TokenUsage, other: &TokenUsage) {
    into.input_tokens += other.input_tokens;
    into.output_tokens += other.output_tokens;
    into.cache_read_tokens += other.cache_read_tokens;
    into.cache_write_tokens += other.cache_write_tokens;
}
