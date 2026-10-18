//! Token usage and cost tracking per agent and provider (phase 6): an append-only ledger,
//! in-memory totals, prices, budgets and read-only readers for the agent CLIs' own logs.

mod aggregate;
mod allow;
mod backfill;
mod budget;
pub(crate) mod cli;
mod estimate;
mod events;
mod ledger;
pub(crate) mod panels;
mod price_cache;
mod pricing;
mod queries;
mod record;
mod summary;
pub(crate) mod time;
mod totals;
mod tracker;

pub use allow::AllowOnceGrant;
pub use backfill::chat_record;
pub use budget::BudgetStatus;
pub use cli::{CliKind, CliRoots};
pub use estimate::{count as count_tokens, is_exact, request_tokens, Estimate};
pub use events::UsageEvent;
pub use queries::{CliSupport, PriceList};
pub use record::{Source, UsageRecord};
pub use summary::{Summary, Totals};
pub use tracker::UsageTracker;
