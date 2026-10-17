// A compact, sortable breakdown table (models, workspaces or panels).
import { createMemo, createSignal, For, Show } from 'solid-js';
import type { UsageGroup } from '../../shared/types';
import { t, type I18nKey } from '../i18n';
import { costText, formatTokens, totalTokens } from './format';
import { SOURCE_LABEL } from './labels';
import { DEFAULT_SORT, nextSort, sortGroups, type SortKey } from './table';
import { scopeLabel } from './usageStore';

const COLUMNS: readonly { key: SortKey; label: I18nKey; numeric: boolean }[] = [
  { key: 'name', label: 'usage.table.name', numeric: false },
  { key: 'requests', label: 'usage.table.requests', numeric: true },
  { key: 'tokens', label: 'usage.table.tokens', numeric: true },
  { key: 'cost', label: 'usage.table.cost', numeric: true },
];

export function UsageTable(props: {
  groups: readonly UsageGroup[];
  name: (group: UsageGroup) => string;
  /** Show at most this many rows (the sidebar view). */
  limit?: number;
  /** Labels CLI cost "API-equivalent". */
  subscription: boolean;
}) {
  // The narrow sidebar leaves out the request count.
  const columns = () =>
    props.limit ? COLUMNS.filter((column) => column.key !== 'requests') : COLUMNS;
  const [sort, setSort] = createSignal(DEFAULT_SORT);
  const rows = createMemo(() => {
    const sorted = sortGroups(props.groups, sort(), props.name);
    return props.limit ? sorted.slice(0, props.limit) : sorted;
  });
  return (
    <Show
      when={props.groups.length > 0}
      fallback={<p class="af-usage-muted">{t('usage.table.empty')}</p>}
    >
      <table class="af-usage-table">
        <thead>
          <tr>
            <For each={columns()}>
              {(column) => (
                <th
                  classList={{ numeric: column.numeric }}
                  aria-sort={
                    sort().key === column.key
                      ? sort().descending
                        ? 'descending'
                        : 'ascending'
                      : undefined
                  }
                >
                  <button type="button" onClick={() => setSort((s) => nextSort(s, column.key))}>
                    {t(column.label)}
                    <Show when={sort().key === column.key}>
                      <span aria-hidden="true">{sort().descending ? ' ▾' : ' ▴'}</span>
                    </Show>
                  </button>
                </th>
              )}
            </For>
          </tr>
        </thead>
        <tbody>
          <For each={rows()}>
            {(group) => {
              const cli = group.source !== 'api';
              return (
                <tr>
                  <td class="af-usage-name" title={props.name(group)}>
                    <span>{props.name(group)}</span>
                    <small>
                      {cli ? t(SOURCE_LABEL[group.source]) : scopeLabel(group.provider)}
                    </small>
                  </td>
                  <Show when={!props.limit}>
                    <td class="numeric">{group.requests}</td>
                  </Show>
                  <td class="numeric">{formatTokens(totalTokens(group))}</td>
                  <td
                    class="numeric"
                    title={
                      group.unpricedTokens > 0
                        ? t('usage.unpriced.hint')
                        : cli && props.subscription
                          ? t('usage.apiEquivalent.hint')
                          : undefined
                    }
                  >
                    {costText(group)}
                    <Show when={cli && props.subscription && group.cost > 0}>
                      <sup class="af-usage-equiv">*</sup>
                    </Show>
                  </td>
                </tr>
              );
            }}
          </For>
        </tbody>
      </table>
    </Show>
  );
}
