// Timeline filtering (plain module, so it can be tested without the DOM).
import type { HandoffEntry } from '../../shared/types';

/** Entries between the pair (both directions), or touching one panel (null = all). */
export function filterEntries(
  entries: readonly HandoffEntry[],
  panel: string | null,
  pair: [string, string] | null,
): HandoffEntry[] {
  return entries.filter((entry) => {
    if (pair)
      return entry.from !== entry.to && pair.includes(entry.from) && pair.includes(entry.to);
    return !panel || entry.from === panel || entry.to === panel;
  });
}
