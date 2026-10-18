// Browse tab search, filters and sort (pure). Every filter narrows; the query matches all
// of its words against name, description, tags, category, source and author.
import type {
  InstallTarget,
  InstalledAgent,
  LibraryItem,
  TemplateKind,
} from '../../../shared/types';

export type SortKey = 'stars' | 'updated' | 'name' | 'tokens';
export type Trust = 'all' | 'verified' | 'unverified';

export interface Filters {
  query: string;
  /** Source id, or '' for all. */
  source: string;
  trust: Trust;
  /** A tag or category, or ''. */
  tag: string;
  target: InstallTarget | '';
  kind: TemplateKind | '';
  sort: SortKey;
}

export const DEFAULT_FILTERS: Filters = {
  query: '',
  source: '',
  trust: 'all',
  tag: '',
  target: '',
  kind: '',
  sort: 'stars',
};

function haystack(item: LibraryItem): string {
  return [
    item.name,
    item.description,
    item.tags.join(' '),
    item.category ?? '',
    item.sourceName,
    item.author,
    item.templateId,
  ]
    .join(' ')
    .toLowerCase();
}

function matches(item: LibraryItem, f: Filters, words: string[]): boolean {
  if (f.source && item.sourceId !== f.source) return false;
  if (f.trust === 'verified' && !item.verified) return false;
  if (f.trust === 'unverified' && item.verified) return false;
  if (f.tag && item.category !== f.tag && !item.tags.includes(f.tag)) return false;
  if (f.target && !item.targets.includes(f.target)) return false;
  if (f.kind && item.kind !== f.kind) return false;
  if (words.length === 0) return true;
  const text = haystack(item);
  return words.every((word) => text.includes(word));
}

const COMPARE: Record<SortKey, (a: LibraryItem, b: LibraryItem) => number> = {
  stars: (a, b) => b.stars - a.stars || a.name.localeCompare(b.name),
  updated: (a, b) =>
    (b.updatedAt ?? '').localeCompare(a.updatedAt ?? '') || a.name.localeCompare(b.name),
  name: (a, b) => a.name.localeCompare(b.name),
  tokens: (a, b) => a.tokens - b.tokens || a.name.localeCompare(b.name),
};

export function filterItems(items: readonly LibraryItem[], f: Filters): LibraryItem[] {
  const words = f.query.toLowerCase().split(/\s+/).filter(Boolean);
  return items.filter((item) => matches(item, f, words)).sort(COMPARE[f.sort]);
}

export interface Facet {
  value: string;
  count: number;
}

/** The most common tags and categories, for the tag filter. */
export function tagFacets(items: readonly LibraryItem[], limit = 60): Facet[] {
  const counts = new Map<string, number>();
  for (const item of items) {
    const tags = new Set([...(item.category ? [item.category] : []), ...item.tags]);
    for (const tag of tags) counts.set(tag, (counts.get(tag) ?? 0) + 1);
  }
  return [...counts]
    .map(([value, count]) => ({ value, count }))
    .sort((a, b) => b.count - a.count || a.value.localeCompare(b.value))
    .slice(0, limit)
    .sort((a, b) => a.value.localeCompare(b.value));
}

/** The library item an installed template came from, when upstream changed since. */
export function updateFor(
  agent: InstalledAgent,
  items: readonly LibraryItem[],
): LibraryItem | undefined {
  const source = agent.template.source;
  if (!source) return undefined;
  return items.find(
    (item) => item.repo === source.repo && item.path === source.path && item.sha !== source.sha,
  );
}
