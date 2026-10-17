//! Token & cost tracking (phase 6): dashboard queries, the pre-send estimate, budgets with
//! "allow once", prices and the CLI readers' status. Live changes arrive on the Channel of
//! `usage_subscribe`. `offset_minutes` is the local UTC offset in minutes east of UTC
//! (`-new Date().getTimezoneOffset()`), so "today" and "this month" are local.

use tauri::ipc::Channel;
use tauri::State;

use crate::chat::{estimate, EstimateRequest};
use crate::conversations::ConversationStore;
use crate::error::AppResult;
use crate::settings::{PriceOverride, Settings, SettingsStore};
use crate::tokens::{
    AllowOnceGrant, BudgetStatus, CliSupport, Estimate, PriceList, Summary, Totals, UsageEvent,
    UsageTracker,
};
use crate::util::run_blocking;

/// Delivers `UsageEvent`s to `on_event` until the next subscribe (or a reload).
#[tauri::command]
pub async fn usage_subscribe(
    tracker: State<'_, UsageTracker>,
    offset_minutes: i32,
    on_event: Channel<UsageEvent>,
) -> AppResult<()> {
    tracker.subscribe(
        offset_minutes,
        Box::new(move |event| {
            let _ = on_event.send(event);
        }),
    );
    Ok(())
}

/// Today / 7 days / 30 days, a daily series and breakdowns by model, workspace and panel.
#[tauri::command]
pub async fn usage_summary(
    tracker: State<'_, UsageTracker>,
    offset_minutes: i32,
) -> AppResult<Summary> {
    let tracker = tracker.inner().clone();
    run_blocking(move || Ok(tracker.summary(offset_minutes))).await
}

/// One panel's totals since `since` (ms; e.g. when the panel opened).
#[tauri::command]
pub async fn usage_panel_totals(
    tracker: State<'_, UsageTracker>,
    panel: String,
    since: u64,
) -> AppResult<Totals> {
    Ok(tracker.panel_totals(&panel, since))
}

/// Tokens and input cost of the request the prompt bar would send now.
#[tauri::command]
pub async fn usage_estimate(
    settings: State<'_, SettingsStore>,
    store: State<'_, ConversationStore>,
    tracker: State<'_, UsageTracker>,
    request: EstimateRequest,
) -> AppResult<Estimate> {
    let settings = settings.get().await;
    let (store, tracker) = (store.inner().clone(), tracker.inner().clone());
    run_blocking(move || {
        let EstimateRequest {
            workspace,
            conversation_id,
            text,
            attachments,
        } = request;
        let conversation = store.get(workspace.as_deref(), &conversation_id)?;
        estimate(&settings, &tracker, &conversation, text, attachments)
    })
    .await
}

/// Every configured budget with its spend and level (0, 80 or 100).
#[tauri::command]
pub async fn usage_budgets(
    tracker: State<'_, UsageTracker>,
    offset_minutes: i32,
) -> AppResult<Vec<BudgetStatus>> {
    Ok(tracker.budgets(offset_minutes))
}

/// A single-use token for `chat_send` (`allowOnce`) past a hard budget stop.
#[tauri::command]
pub async fn usage_allow_once(
    tracker: State<'_, UsageTracker>,
    provider_id: String,
) -> AppResult<AllowOnceGrant> {
    Ok(tracker.allow_once(&provider_id))
}

#[tauri::command]
pub async fn usage_prices(tracker: State<'_, UsageTracker>) -> AppResult<PriceList> {
    Ok(tracker.prices())
}

/// Replaces the user's price overrides (kept in settings) and returns the saved settings.
#[tauri::command]
pub async fn usage_set_prices(
    settings: State<'_, SettingsStore>,
    tracker: State<'_, UsageTracker>,
    prices: Vec<PriceOverride>,
) -> AppResult<Settings> {
    let mut next = settings.get().await;
    next.usage.prices = prices;
    let saved = settings.save(next).await?;
    tracker.apply_settings(&saved.usage);
    Ok(saved)
}

/// Whether each agent CLI's usage can be read and is being read.
#[tauri::command]
pub async fn usage_cli_status(tracker: State<'_, UsageTracker>) -> AppResult<Vec<CliSupport>> {
    let tracker = tracker.inner().clone();
    run_blocking(move || Ok(tracker.cli_status())).await
}
