// The Browse tab's state for the whole session: search and filters, how many cards are
// shown, the open item and the scroll position survive switching tabs or closing the
// panel. Nothing here is saved to disk.
import { createSignal } from 'solid-js';
import { createStore } from 'solid-js/store';
import type { LibraryItem } from '../../../shared/types';
import { DEFAULT_FILTERS, type Filters } from './filters';

export const PAGE = 40;

const [filters, setFilters] = createStore<Filters>({ ...DEFAULT_FILTERS });
const [limit, setLimit] = createSignal(PAGE);
const [selected, setSelected] = createSignal<LibraryItem | null>(null);
let scrollTop = 0;

export { filters, limit, selected, setSelected };

/** Changes one filter; the list starts again at the first page. */
export function setFilter<K extends keyof Filters>(key: K, value: Filters[K]): void {
  setFilters(key, value);
  setLimit(PAGE);
  scrollTop = 0;
}

export function showMore(): void {
  setLimit((n) => n + PAGE);
}

export function rememberScroll(top: number): void {
  scrollTop = top;
}

export function savedScroll(): number {
  return scrollTop;
}

/** Back to the defaults (tests; a future "clear filters" button). */
export function resetBrowse(): void {
  setFilters({ ...DEFAULT_FILTERS });
  setLimit(PAGE);
  setSelected(null);
  scrollTop = 0;
}
