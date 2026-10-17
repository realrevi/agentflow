// The app's one usage subscription (Channel of usage_subscribe) and what hangs off it: the
// summary (status bar, Usage view and panel), per-panel listeners (corner badges) and
// budget alerts (toast, plus the affected panels' characters). Event driven, no polling.
import { createSignal } from 'solid-js';
import { debounce } from '../../shared/debounce';
import { ipc } from '../../shared/ipc';
import { showToast } from '../../shared/toast';
import type { BudgetStatus, UsageEvent, UsageSource, UsageSummary } from '../../shared/types';
import { reportError, t } from '../i18n';
import { providerName } from '../providers';
import { settings } from '../settings/settingsStore';
import { formatCost } from './format';
import { SOURCE_LABEL } from './labels';

const SUMMARY_DELAY_MS = 400;
const CLI_SCOPES = ['claude-code', 'codex', 'gemini-cli'];

const [summary, setSummary] = createSignal<UsageSummary | null>(null);
const panelListeners = new Map<string, Set<() => void>>();
const repriceListeners = new Set<() => void>();
const budgetListeners = new Set<(status: BudgetStatus) => void>();
let started = false;

/** The last loaded summary (`null` until the first one arrives). */
export { summary as usageSummary };

async function loadSummary(): Promise<void> {
  try {
    setSummary(await ipc.usageSummary());
  } catch (err) {
    reportError('error.usageLoad', err);
  }
}

const refreshSummary = debounce(() => void loadSummary(), SUMMARY_DELAY_MS);

/** Reloads the summary now (e.g. when the Usage panel opens). */
export function reloadUsage(): Promise<void> {
  return loadSummary();
}

function notifyPanel(panel: string): void {
  for (const listener of panelListeners.get(panel) ?? []) listener();
}

/** Applies one event; exported for tests. */
export function handleUsageEvent(event: UsageEvent): void {
  switch (event.type) {
    case 'changed':
      for (const panel of event.panels) notifyPanel(panel);
      refreshSummary();
      break;
    case 'repriced':
      for (const listener of repriceListeners) listener();
      for (const panel of panelListeners.keys()) notifyPanel(panel);
      refreshSummary();
      break;
    case 'budget':
      announceBudget(event.status);
      for (const listener of budgetListeners) listener(event.status);
      break;
  }
}

/** Subscribes once at startup and loads the first summary. */
export async function startUsage(): Promise<void> {
  if (started) return;
  started = true;
  try {
    await ipc.usageSubscribe(handleUsageEvent);
  } catch (err) {
    started = false;
    reportError('error.usageLoad', err);
  }
  await loadSummary();
}

/** Calls `listener` when `panel` got new usage or prices changed. Returns an unsubscribe. */
export function onPanelUsage(panel: string, listener: () => void): () => void {
  const listeners = panelListeners.get(panel) ?? new Set();
  listeners.add(listener);
  panelListeners.set(panel, listeners);
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0) panelListeners.delete(panel);
  };
}

/** Calls `listener` when prices changed (price lists, estimates). */
export function onRepriced(listener: () => void): () => void {
  repriceListeners.add(listener);
  return () => repriceListeners.delete(listener);
}

/** Calls `listener` for every budget crossing (80% or 100%). */
export function onBudgetAlert(listener: (status: BudgetStatus) => void): () => void {
  budgetListeners.add(listener);
  return () => budgetListeners.delete(listener);
}

export function isCliScope(scope: string): boolean {
  return CLI_SCOPES.includes(scope);
}

/** "All sources", a provider's name or a CLI name. */
export function scopeLabel(scope: string): string {
  if (scope === 'all') return t('usage.scope.all');
  if (isCliScope(scope)) return t(SOURCE_LABEL[scope as UsageSource]);
  const entry = settings.providers.find((provider) => provider.id === scope);
  return entry ? providerName(entry) : scope;
}

/** Does a budget `scope` cover usage of `provider` (an entry id or a CLI key)? */
export function budgetCovers(scope: string, provider: string | null | undefined): boolean {
  return scope === 'all' || (provider !== null && provider !== undefined && scope === provider);
}

function announceBudget(status: BudgetStatus): void {
  const params = {
    scope: scopeLabel(status.scope),
    period: t(status.period === 'daily' ? 'usage.period.daily' : 'usage.period.monthly'),
    spent: formatCost(status.spent),
    limit: formatCost(status.limit),
  };
  const reached = status.level >= 100;
  const lines = [t('usage.budget.spent', params)];
  if (reached) {
    lines.push(
      t(isCliScope(status.scope) ? 'usage.budget.cliWarnOnly' : 'usage.budget.hardStopHint'),
    );
  }
  showToast({
    kind: reached ? 'error' : 'info',
    title: t(reached ? 'usage.budget.reached' : 'usage.budget.warning', params),
    detail: lines.join(' '),
  });
}
