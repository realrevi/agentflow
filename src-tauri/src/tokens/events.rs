//! What the frontend's usage Channel (`usage_subscribe`) receives.

use serde::Serialize;

use super::budget::BudgetStatus;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UsageEvent {
    /// New usage was recorded; `panels` lists the panels it belongs to (may be empty).
    Changed { panels: Vec<String> },
    /// Prices, free providers or budgets changed: every cost may differ.
    Repriced,
    /// A budget crossed 80% or 100% (sent once per level and period).
    Budget { status: BudgetStatus },
}

/// Receives usage events (the Channel of the last `usage_subscribe`).
pub type UsageSink = Box<dyn Fn(UsageEvent) + Send + Sync>;
