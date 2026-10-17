// The usage dashboard, in the sidebar (compact) and as a dock panel: totals for today /
// 7 / 30 days, the daily chart, cache savings and unpriced tokens, breakdowns by model,
// workspace and panel, and the CLI readers' status. Live via the usage store (no polling).
import './usage.css';
import { createMemo, createSignal, For, Show } from 'solid-js';
import type { Totals, UsageGroup } from '../../shared/types';
import { t, type I18nKey } from '../i18n';
import { settings } from '../settings/settingsStore';
import { defaultMetric, type ChartMetric } from './chart';
import { CliStatusList } from './CliStatusList';
import { costText, formatCost, formatTokens, totalTokens } from './format';
import { UsageChart } from './UsageChart';
import { UsageTable } from './UsageTable';
import { usageSummary } from './usageStore';

type Period = 'today' | 'week' | 'month';
type Breakdown = 'models' | 'workspaces' | 'panels';

const PERIODS: readonly { id: Period; label: I18nKey }[] = [
  { id: 'today', label: 'usage.period.today' },
  { id: 'week', label: 'usage.period.week' },
  { id: 'month', label: 'usage.period.month' },
];
const BREAKDOWNS: readonly { id: Breakdown; label: I18nKey }[] = [
  { id: 'models', label: 'usage.breakdown.models' },
  { id: 'workspaces', label: 'usage.breakdown.workspaces' },
  { id: 'panels', label: 'usage.breakdown.panels' },
];
const COMPACT_ROWS = 5;

export interface UsageDashboardProps {
  /** The sidebar version: fewer rows and a button for the full panel. */
  compact?: boolean;
  /** The title of an open panel (closed panels show their id). */
  panelTitle: (id: string) => string | undefined;
  onOpenPanel?: () => void;
  onOpenSettings: () => void;
}

export function UsageDashboard(props: UsageDashboardProps) {
  const [period, setPeriod] = createSignal<Period>('today');
  const [breakdown, setBreakdown] = createSignal<Breakdown>('models');
  const [metric, setMetric] = createSignal<ChartMetric | null>(null);
  const summary = usageSummary;
  const totals = createMemo((): Totals | null => summary()?.[period()] ?? null);
  const chartMetric = () => metric() ?? defaultMetric(summary()?.month ?? emptyTotals);
  const subscription = () => settings.usage.cliSubscription;

  const groupName = (kind: Breakdown) => (group: UsageGroup) => {
    if (kind === 'workspaces') return group.key ? baseName(group.key) : t('usage.noWorkspace');
    if (kind === 'panels') return props.panelTitle(group.key) ?? group.key;
    return group.key;
  };

  return (
    <div class="af-usage" classList={{ compact: props.compact ?? false }}>
      <Show when={summary()} fallback={<p class="af-usage-muted">{t('panel.loading')}</p>}>
        {(current) => (
          <Show when={current().month.requests > 0} fallback={<EmptyState />}>
            <div class="af-segmented" role="radiogroup" aria-label={t('usage.period.label')}>
              <For each={PERIODS}>
                {(entry) => (
                  <button
                    type="button"
                    class="af-segment"
                    role="radio"
                    aria-checked={period() === entry.id}
                    onClick={() => setPeriod(entry.id)}
                  >
                    {t(entry.label)}
                  </button>
                )}
              </For>
            </div>
            <Show when={totals()}>
              {(shown) => <TotalsCards totals={shown()} subscription={subscription()} />}
            </Show>

            <div class="af-usage-heading">
              <span>{t('usage.chart.title')}</span>
              <button
                type="button"
                class="af-usage-link"
                onClick={() => setMetric(chartMetric() === 'cost' ? 'tokens' : 'cost')}
              >
                {t(chartMetric() === 'cost' ? 'usage.chart.showTokens' : 'usage.chart.showCost')}
              </button>
            </div>
            <UsageChart days={current().days} metric={chartMetric()} />

            <Show when={(totals()?.cacheSavings ?? 0) > 0}>
              <p class="af-usage-line ok">
                {t('usage.cacheSavings', { amount: formatCost(totals()?.cacheSavings ?? 0) })}
              </p>
            </Show>
            <Show when={(totals()?.unpricedTokens ?? 0) > 0}>
              <p class="af-usage-line">
                {t('usage.unpriced', { count: formatTokens(totals()?.unpricedTokens ?? 0) })}{' '}
                <button type="button" class="af-usage-link" onClick={() => props.onOpenSettings()}>
                  {t('usage.unpriced.add')}
                </button>
              </p>
            </Show>

            <div class="af-usage-heading">
              <span>{t('usage.breakdown.title')}</span>
            </div>
            <div class="af-usage-tabs" role="tablist">
              <For each={BREAKDOWNS}>
                {(entry) => (
                  <button
                    type="button"
                    role="tab"
                    aria-selected={breakdown() === entry.id}
                    onClick={() => setBreakdown(entry.id)}
                  >
                    {t(entry.label)}
                  </button>
                )}
              </For>
            </div>
            <UsageTable
              groups={current()[breakdown()]}
              name={groupName(breakdown())}
              subscription={subscription()}
              {...(props.compact ? { limit: COMPACT_ROWS } : {})}
            />
            <Show when={current().compact.requests > 0}>
              <p class="af-usage-line">
                {t('usage.compactCost', {
                  count: current().compact.requests,
                  cost: costText(current().compact),
                })}
              </p>
            </Show>
            <Show when={subscription() && current().month.cliCost > 0}>
              <p class="af-usage-line af-usage-muted">* {t('usage.apiEquivalent.hint')}</p>
            </Show>

            <div class="af-usage-heading">
              <span>{t('usage.cli.title')}</span>
            </div>
            <CliStatusList />
          </Show>
        )}
      </Show>
      <Show when={props.compact && props.onOpenPanel}>
        {(open) => (
          <button type="button" class="af-button secondary small af-usage-open" onClick={open()}>
            {t('usage.openPanel')}
          </button>
        )}
      </Show>
    </div>
  );
}

function TotalsCards(props: { totals: Totals; subscription: boolean }) {
  return (
    <div class="af-usage-cards">
      <div class="af-usage-card">
        <span class="af-usage-card-value">{formatTokens(totalTokens(props.totals))}</span>
        <span class="af-usage-card-label">{t('usage.card.tokens')}</span>
      </div>
      <div class="af-usage-card">
        <span class="af-usage-card-value">{props.totals.requests}</span>
        <span class="af-usage-card-label">{t('usage.card.requests')}</span>
      </div>
      <div class="af-usage-card">
        <span class="af-usage-card-value">{costText(props.totals)}</span>
        <span class="af-usage-card-label">
          {props.subscription && props.totals.cliCost > 0
            ? t('usage.card.costEquiv', { amount: formatCost(props.totals.cliCost) })
            : t('usage.card.cost')}
        </span>
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div class="af-usage-empty">
      <p>{t('usage.empty.title')}</p>
      <p class="af-usage-muted">{t('usage.empty.hint')}</p>
      <CliStatusList />
    </div>
  );
}

const emptyTotals: Totals = {
  inputTokens: 0,
  outputTokens: 0,
  cacheReadTokens: 0,
  cacheWriteTokens: 0,
  requests: 0,
  cost: 0,
  unpricedTokens: 0,
  cacheSavings: 0,
  cliCost: 0,
};

function baseName(path: string): string {
  return (
    path
      .replace(/[\\/]+$/, '')
      .split(/[\\/]/)
      .pop() || path
  );
}
