import { describe, expect, it } from 'vitest';
import type { InstalledAgent, LibraryItem } from '../../../shared/types';
import { DEFAULT_FILTERS, filterItems, tagFacets, updateFor } from './filters';

function item(over: Partial<LibraryItem>): LibraryItem {
  return {
    id: 'src:a.md',
    sourceId: 'src',
    sourceName: 'Source',
    repo: 'acme/agents',
    verified: true,
    path: 'a.md',
    url: 'https://github.com/acme/agents',
    format: 'claudeSubagent',
    kind: 'agent',
    templateId: 'a',
    name: 'A',
    description: '',
    author: 'acme',
    license: 'MIT',
    tags: [],
    category: null,
    targets: ['claudeCode', 'codex', 'gemini', 'cursor', 'agentflow'],
    native: ['claudeCode'],
    tokens: 100,
    flags: [],
    files: [],
    sha: 'sha-1',
    commit: 'c1',
    updatedAt: '2026-10-01T00:00:00Z',
    stars: 10,
    ...over,
  };
}

const ITEMS = [
  item({
    id: '1',
    name: 'Code Reviewer',
    description: 'Reviews diffs',
    tags: ['quality'],
    stars: 5,
  }),
  item({
    id: '2',
    name: 'PDF Forms',
    kind: 'skill',
    targets: ['claudeCode', 'codex'],
    category: 'docs',
    tokens: 900,
    stars: 50,
    updatedAt: '2026-09-01T00:00:00Z',
  }),
  item({
    id: '3',
    name: 'React Rules',
    kind: 'rule',
    sourceId: 'mine',
    verified: false,
    tokens: 20,
    stars: 1,
    updatedAt: '2026-10-10T00:00:00Z',
  }),
];

const ids = (items: LibraryItem[]) => items.map((i) => i.id);

describe('library filters', () => {
  it('sorts by stars by default, then by name, updates and size', () => {
    expect(ids(filterItems(ITEMS, DEFAULT_FILTERS))).toEqual(['2', '1', '3']);
    expect(ids(filterItems(ITEMS, { ...DEFAULT_FILTERS, sort: 'name' }))).toEqual(['1', '2', '3']);
    expect(ids(filterItems(ITEMS, { ...DEFAULT_FILTERS, sort: 'updated' }))).toEqual([
      '3',
      '1',
      '2',
    ]);
    expect(ids(filterItems(ITEMS, { ...DEFAULT_FILTERS, sort: 'tokens' }))).toEqual([
      '3',
      '1',
      '2',
    ]);
  });

  it('matches every word of the query and narrows by each filter', () => {
    const f = (over: Partial<typeof DEFAULT_FILTERS>) =>
      ids(filterItems(ITEMS, { ...DEFAULT_FILTERS, ...over }));
    expect(f({ query: 'review DIFFS' })).toEqual(['1']);
    expect(f({ query: 'review pdf' })).toEqual([]);
    expect(f({ source: 'mine' })).toEqual(['3']);
    expect(f({ trust: 'verified' })).toEqual(['2', '1']);
    expect(f({ trust: 'unverified' })).toEqual(['3']);
    expect(f({ tag: 'docs' })).toEqual(['2']);
    expect(f({ tag: 'quality' })).toEqual(['1']);
    expect(f({ target: 'agentflow' })).toEqual(['1', '3']);
    expect(f({ kind: 'skill' })).toEqual(['2']);
  });

  it('lists tags and categories with counts', () => {
    expect(tagFacets(ITEMS)).toEqual([
      { value: 'docs', count: 1 },
      { value: 'quality', count: 1 },
    ]);
  });

  it('offers an update when the upstream sha differs', () => {
    const agent = (sha: string) =>
      ({
        template: {
          source: { repo: 'acme/agents', path: 'a.md', sha, commit: 'c', license: 'MIT' },
        },
      }) as unknown as InstalledAgent;
    const items = [item({ path: 'a.md', sha: 'new' })];
    expect(updateFor(agent('old'), items)?.sha).toBe('new');
    expect(updateFor(agent('new'), items)).toBeUndefined();
    expect(updateFor({ template: {} } as unknown as InstalledAgent, items)).toBeUndefined();
  });
});
