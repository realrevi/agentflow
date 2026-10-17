// Sorting for the breakdown tables (by name, requests, tokens or cost).
import type { UsageGroup } from '../../shared/types';
import { totalTokens } from './format';

export type SortKey = 'name' | 'requests' | 'tokens' | 'cost';

export interface Sort {
  key: SortKey;
  /** Numbers start high to low, names A to Z. */
  descending: boolean;
}

export const DEFAULT_SORT: Sort = { key: 'cost', descending: true };

/** Clicking a header: the same column flips, a new one starts in its natural order. */
export function nextSort(current: Sort, key: SortKey): Sort {
  if (current.key === key) return { key, descending: !current.descending };
  return { key, descending: key !== 'name' };
}

function value(group: UsageGroup, key: Exclude<SortKey, 'name'>): number {
  switch (key) {
    case 'requests':
      return group.requests;
    case 'tokens':
      return totalTokens(group);
    case 'cost':
      return group.cost;
  }
}

export function sortGroups(
  groups: readonly UsageGroup[],
  sort: Sort,
  name: (group: UsageGroup) => string,
): UsageGroup[] {
  const sign = sort.descending ? -1 : 1;
  return [...groups].sort((a, b) => {
    const order =
      sort.key === 'name'
        ? name(a).localeCompare(name(b))
        : value(a, sort.key) - value(b, sort.key);
    // Ties: more tokens first, then by name, so the order never jumps around.
    return sign * order || totalTokens(b) - totalTokens(a) || name(a).localeCompare(name(b));
  });
}
