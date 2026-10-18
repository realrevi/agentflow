// The Browse tab: the community library's cards with search, filters and sort, the
// refresh status (rate limit, offline), the sources, and an item's detail view. Install
// and Customize with AI hand the item's template to the wizard.
import './library.css';
import { createMemo, createSignal, For, onMount, Show } from 'solid-js';
import { createStore } from 'solid-js/store';
import type { AgentTemplate, LibraryItem, TemplateKind } from '../../../shared/types';
import { t } from '../../i18n';
import { TARGETS } from '../wizard/answers';
import { DEFAULT_FILTERS, filterItems, tagFacets, type Filters, type SortKey } from './filters';
import { ItemCard } from './ItemCard';
import { ItemDetail } from './ItemDetail';
import { KIND_LABELS, resetTime } from './labels';
import { catalog, openLibrary, refreshing, refreshLibrary } from './libraryStore';
import { SourcesSection } from './SourcesSection';
import { Button } from '../../../shared/ui/Button';
import { Input, Select } from '../../../shared/ui/Field';

const PAGE = 40;
const SORTS: readonly {
  id: SortKey;
  label:
    'library.sort.stars' | 'library.sort.updated' | 'library.sort.name' | 'library.sort.tokens';
}[] = [
  { id: 'stars', label: 'library.sort.stars' },
  { id: 'updated', label: 'library.sort.updated' },
  { id: 'name', label: 'library.sort.name' },
  { id: 'tokens', label: 'library.sort.tokens' },
];
const KINDS: readonly TemplateKind[] = ['agent', 'skill', 'rule', 'command'];

export function BrowseTab(props: {
  workspace: string | null;
  onInstall: (template: AgentTemplate, item: LibraryItem) => void;
}) {
  onMount(() => void openLibrary());
  const [filters, setFilters] = createStore<Filters>({ ...DEFAULT_FILTERS });
  const [limit, setLimit] = createSignal(PAGE);
  const [selected, setSelected] = createSignal<LibraryItem | null>(null);
  const [showSources, setShowSources] = createSignal(false);
  const items = createMemo(() => filterItems(catalog()?.items ?? [], { ...filters }));
  const tags = createMemo(() => tagFacets(catalog()?.items ?? []));
  const set = <K extends keyof Filters>(key: K, value: Filters[K]) => {
    setFilters(key, value);
    setLimit(PAGE);
  };
  const limited = () =>
    catalog()?.sources.find((s) => s.error?.kind === 'rateLimited')?.error?.resetAt ?? null;
  const offline = () =>
    (catalog()?.sources ?? []).some((s) => s.error?.kind === 'network') && !limited();

  return (
    <Show
      when={selected()}
      fallback={
        <section class="af-wizard-step af-lib">
          <div class="af-lib-status">
            <span class="af-installer-hint">
              {catalog()?.refreshedAt
                ? t('library.status', {
                    items: catalog()?.items.length ?? 0,
                    sources: catalog()?.sources.length ?? 0,
                    time: new Date(catalog()?.refreshedAt ?? 0).toLocaleString(),
                  })
                : t('library.neverRefreshed')}
            </span>
            <Button
              variant="secondary"
              disabled={refreshing()}
              onClick={() => void refreshLibrary()}
            >
              {refreshing() ? t('library.refreshing') : t('library.refresh')}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              aria-expanded={showSources()}
              onClick={() => setShowSources((on) => !on)}
            >
              {t('library.sources')}
            </Button>
          </div>
          <Show when={limited()}>
            {(at) => (
              <p class="af-installer-warning">
                {t('library.rateLimited', { time: resetTime(at()) })}
              </p>
            )}
          </Show>
          <Show when={offline()}>
            <p class="af-installer-warning">{t('library.offline')}</p>
          </Show>
          <Show when={showSources()}>
            <SourcesSection catalog={catalog()} />
          </Show>
          <div class="af-lib-filters">
            <Input
              class="af-lib-search"
              type="search"
              aria-label={t('library.search')}
              placeholder={t('library.search')}
              value={filters.query}
              onInput={(e) => set('query', e.currentTarget.value)}
            />
            <Select
              aria-label={t('library.filter.source')}
              value={filters.source}
              onChange={(e) => set('source', e.currentTarget.value)}
            >
              <option value="">{t('library.filter.allSources')}</option>
              <For each={catalog()?.sources ?? []}>
                {(source) => <option value={source.id}>{source.name}</option>}
              </For>
            </Select>
            <Select
              aria-label={t('library.filter.trust')}
              value={filters.trust}
              onChange={(e) => set('trust', e.currentTarget.value as Filters['trust'])}
            >
              <option value="all">{t('library.filter.anyTrust')}</option>
              <option value="verified">{t('library.verified')}</option>
              <option value="unverified">{t('library.unverified')}</option>
            </Select>
            <Select
              aria-label={t('library.filter.kind')}
              value={filters.kind}
              onChange={(e) => set('kind', e.currentTarget.value as Filters['kind'])}
            >
              <option value="">{t('library.filter.anyKind')}</option>
              <For each={KINDS}>
                {(kind) => <option value={kind}>{t(KIND_LABELS[kind])}</option>}
              </For>
            </Select>
            <Select
              aria-label={t('library.filter.target')}
              value={filters.target}
              onChange={(e) => set('target', e.currentTarget.value as Filters['target'])}
            >
              <option value="">{t('library.filter.anyTarget')}</option>
              <For each={TARGETS}>
                {(target) => <option value={target.id}>{t(target.label)}</option>}
              </For>
            </Select>
            <Select
              aria-label={t('library.filter.tag')}
              value={filters.tag}
              onChange={(e) => set('tag', e.currentTarget.value)}
            >
              <option value="">{t('library.filter.anyTag')}</option>
              <For each={tags()}>
                {(tag) => (
                  <option value={tag.value}>
                    {tag.value} ({tag.count})
                  </option>
                )}
              </For>
            </Select>
            <Select
              aria-label={t('library.sort')}
              value={filters.sort}
              onChange={(e) => set('sort', e.currentTarget.value as SortKey)}
            >
              <For each={SORTS}>{(sort) => <option value={sort.id}>{t(sort.label)}</option>}</For>
            </Select>
          </div>
          <p class="af-installer-hint">{t('library.count', { count: items().length })}</p>
          <div class="af-lib-grid">
            <For each={items().slice(0, limit())}>
              {(item) => <ItemCard item={item} onOpen={() => setSelected(item)} />}
            </For>
          </div>
          <Show when={items().length > limit()}>
            <Button variant="secondary" onClick={() => setLimit((n) => n + PAGE)}>
              {t('library.more', { count: items().length - limit() })}
            </Button>
          </Show>
        </section>
      }
    >
      {(item) => (
        <ItemDetail
          item={item()}
          workspace={props.workspace}
          onBack={() => setSelected(null)}
          onInstall={props.onInstall}
        />
      )}
    </Show>
  );
}
