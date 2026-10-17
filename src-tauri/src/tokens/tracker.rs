//! Managed state for token & cost tracking: the ledger on disk, the in-memory totals, prices,
//! budgets, the CLI log readers and which panel ran which agent CLI. Queries live in
//! `queries.rs`. Lock order: `cli` before `state`, never the other way round.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};

use super::aggregate::{Aggregator, DAY_MS, WINDOW_DAYS};
use super::allow::AllowOnce;
use super::backfill::{conversation_records, MARKER};
use super::budget::Notified;
use super::cli::{self, CliKind, CliReaders, CliRoots, LineUsage};
use super::events::{UsageEvent, UsageSink};
use super::ledger::LedgerFiles;
use super::panels::{PanelRegistry, PanelSession};
use super::price_cache;
use super::pricing::PriceBook;
use super::record::UsageRecord;
use super::time::month_key;
use crate::error::AppResult;
use crate::settings::{Settings, UsageSettings};
use crate::util::{lock, now_ms};

const OFFSETS_FILE: &str = "cli-offsets.json";

#[derive(Clone)]
pub struct UsageTracker {
    pub(super) inner: Arc<Inner>,
}

pub(super) struct Inner {
    /// `<app data>/usage`
    pub dir: PathBuf,
    pub ledger: LedgerFiles,
    pub state: Mutex<State>,
    pub cli: Mutex<CliReaders>,
}

pub(super) struct State {
    pub agg: Aggregator,
    pub book: PriceBook,
    pub panels: PanelRegistry,
    pub usage: UsageSettings,
    pub notified: Notified,
    /// Minutes east of UTC, as the frontend last reported (local days and months).
    pub offset: i32,
    pub allow: AllowOnce,
    /// Context windows from models endpoints, by `provider|model`.
    pub windows: HashMap<String, u64>,
    pub sink: Option<UsageSink>,
    last_prune: u64,
}

impl UsageTracker {
    /// `dir`: `<app data>/usage`. Nothing is read until `start`.
    pub fn new(dir: PathBuf, roots: CliRoots) -> Self {
        let state = State {
            agg: Aggregator::default(),
            book: PriceBook::shipped(),
            panels: PanelRegistry::default(),
            usage: UsageSettings::default(),
            notified: Notified::default(),
            offset: 0,
            allow: AllowOnce::default(),
            windows: HashMap::new(),
            sink: None,
            last_prune: 0,
        };
        let cli = CliReaders::new(roots, dir.join(OFFSETS_FILE));
        Self {
            inner: Arc::new(Inner {
                ledger: LedgerFiles::new(dir.clone()),
                dir,
                state: Mutex::new(state),
                cli: Mutex::new(cli),
            }),
        }
    }

    /// App start (blocking, on a background thread): loads the ledger, backfills it once
    /// from stored chats, then reads what the CLIs logged meanwhile and starts watching.
    pub fn start(&self, settings: &Settings, conversations: &Path) {
        let now = now_ms();
        {
            let mut state = lock(&self.inner.state);
            let since = month_key(now.saturating_sub(WINDOW_DAYS * DAY_MS));
            for record in self.inner.ledger.load(&since) {
                state.agg.add(&record, now);
            }
            state.agg.prune(now);
            state.last_prune = now;
            for (model, price) in price_cache::load(&self.inner.dir.join(price_cache::FILE)) {
                state.book.set_dynamic(&model, price);
            }
            for workspace in settings
                .current_workspace
                .iter()
                .chain(&settings.recent_workspaces)
            {
                state.panels.note_workspace(workspace);
            }
        }
        self.apply_settings(&settings.usage);
        if let Err(err) = self.backfill_once(conversations) {
            tracing::warn!("usage backfill failed: {err}");
        }
        let statuses = self.budget_statuses(now);
        let (day, month) = self.period_keys(now);
        lock(&self.inner.state)
            .notified
            .baseline(&statuses, day, &month);
    }

    /// Adds records the ledger does not have yet (by id); returns how many were new.
    pub fn record(&self, records: Vec<UsageRecord>) -> AppResult<usize> {
        let now = now_ms();
        let mut state = lock(&self.inner.state);
        let mut batch: Vec<UsageRecord> = Vec::new();
        for record in records {
            let repeated = !record.id.is_empty()
                && batch
                    .iter()
                    .any(|r| r.source == record.source && r.id == record.id);
            if state.agg.is_new(&record) && !repeated {
                batch.push(record);
            }
        }
        if batch.is_empty() {
            return Ok(0);
        }
        self.inner.ledger.append(&batch)?;
        let mut panels: Vec<String> = Vec::new();
        for record in &batch {
            state.agg.add(record, now);
            if let Some(panel) = &record.panel {
                if !panels.contains(panel) {
                    panels.push(panel.clone());
                }
            }
        }
        if now.saturating_sub(state.last_prune) > DAY_MS {
            state.agg.prune(now);
            state.last_prune = now;
        }
        drop(state);
        self.emit(UsageEvent::Changed { panels });
        self.check_budgets(now);
        Ok(batch.len())
    }

    /// Imports stored chats once; the marker file keeps it from ever running again.
    pub fn backfill_once(&self, conversations: &Path) -> AppResult<usize> {
        if self.inner.ledger.has_marker(MARKER) {
            return Ok(0);
        }
        let added = self.record(conversation_records(conversations))?;
        self.inner.ledger.write_marker(MARKER)?;
        tracing::info!("usage backfill: {added} chat replies imported");
        Ok(added)
    }

    /// Settings changed: prices, budgets, CLI reading on/off.
    pub fn apply_settings(&self, usage: &UsageSettings) {
        let changed = {
            let mut state = lock(&self.inner.state);
            let changed = state.usage != *usage;
            state.book.apply_settings(usage);
            state.usage = usage.clone();
            changed
        };
        self.set_cli_tracking(usage.track_cli);
        if changed {
            self.emit(UsageEvent::Repriced);
            self.check_budgets(now_ms());
        }
    }

    /// The frontend's usage Channel (replaces the previous one) and its UTC offset.
    pub fn subscribe(&self, offset_minutes: i32, sink: UsageSink) {
        let mut state = lock(&self.inner.state);
        state.offset = offset_minutes;
        state.sink = Some(sink);
    }

    pub fn note_workspace(&self, workspace: &str) {
        lock(&self.inner.state).panels.note_workspace(workspace);
    }

    /// An agent CLI started in a panel (call before spawning, so a fast exit finds it).
    pub fn panel_started(&self, panel: &str, kind: CliKind, cwd: &Path) {
        lock(&self.inner.state).panels.start(PanelSession {
            panel: panel.to_owned(),
            kind,
            cwd: cwd.to_path_buf(),
            workspace: None,
            started: now_ms(),
            ended: None,
        });
    }

    pub fn panel_ended(&self, panel: &str) {
        lock(&self.inner.state).panels.end(panel, now_ms());
    }

    /// App exit: stops the watcher and saves the read offsets.
    pub fn shutdown(&self) {
        let mut cli = lock(&self.inner.cli);
        cli.watch = None;
        cli.save();
    }

    pub(super) fn emit(&self, event: UsageEvent) {
        // Channel sends never block, so the sink is called under the lock.
        if let Some(sink) = &lock(&self.inner.state).sink {
            sink(event);
        }
    }

    fn set_cli_tracking(&self, on: bool) {
        let mut cli = lock(&self.inner.cli);
        if !on {
            cli.watch = None;
            return;
        }
        if cli.watch.is_some() {
            return;
        }
        let weak: Weak<Inner> = Arc::downgrade(&self.inner);
        cli.watch = cli::watch(&cli.roots, move |paths| {
            if let Some(inner) = weak.upgrade() {
                UsageTracker { inner }.ingest(&paths);
            }
        });
        // Catch up on what the CLIs logged while the app was closed (or tracking was off).
        let since = now_ms().saturating_sub(WINDOW_DAYS * DAY_MS);
        let files = cli.recent_files(since);
        drop(cli);
        let tracker = self.clone();
        let spawned = std::thread::Builder::new()
            .name("agentflow-cli-scan".into())
            .spawn(move || {
                for chunk in files.chunks(64) {
                    tracker.ingest(chunk);
                }
            });
        if let Err(err) = spawned {
            tracing::warn!("cannot scan CLI sessions: {err}");
        }
    }

    /// Reads new usage from changed CLI session files into the ledger.
    pub fn ingest(&self, paths: &[PathBuf]) {
        let mut cli = lock(&self.inner.cli);
        let found = cli.read(paths);
        if found.is_empty() {
            cli.save();
            return;
        }
        let records = {
            let state = lock(&self.inner.state);
            found
                .into_iter()
                .map(|(kind, usage)| cli_record(&state.panels, kind, usage))
                .collect()
        };
        match self.record(records) {
            // Offsets move on only once the records are safely in the ledger.
            Ok(_) => cli.save(),
            Err(err) => tracing::warn!("cannot store CLI usage: {err}"),
        }
    }
}

fn cli_record(panels: &PanelRegistry, kind: CliKind, usage: LineUsage) -> UsageRecord {
    let at = if usage.at == 0 { now_ms() } else { usage.at };
    let (panel, workspace) = panels.attribute(kind, usage.cwd.as_deref(), at);
    let source = kind.source();
    UsageRecord {
        at,
        source,
        provider: source.key().to_owned(),
        preset: kind.vendor().to_owned(),
        model: usage.model.unwrap_or_else(|| "unknown".into()),
        workspace,
        panel,
        id: usage.id,
        kind: String::new(),
        tokens: usage.tokens,
    }
}

#[cfg(test)]
#[path = "tracker_tests.rs"]
mod tests;
