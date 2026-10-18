import { describe, expect, it } from 'vitest';
import {
  filters,
  limit,
  PAGE,
  rememberScroll,
  resetBrowse,
  savedScroll,
  setFilter,
  showMore,
} from './browseState';

describe('Browse state', () => {
  it('keeps search, filters, page size and scroll for the session', () => {
    resetBrowse();
    setFilter('query', 'review');
    setFilter('kind', 'skill');
    showMore();
    rememberScroll(480);
    // A tab switch remounts the Browse tab: it reads the same module state again.
    expect(filters.query).toBe('review');
    expect(filters.kind).toBe('skill');
    expect(limit()).toBe(PAGE * 2);
    expect(savedScroll()).toBe(480);
  });

  it('starts at the first page and the top when a filter changes', () => {
    resetBrowse();
    showMore();
    rememberScroll(300);
    setFilter('sort', 'name');
    expect(limit()).toBe(PAGE);
    expect(savedScroll()).toBe(0);
    expect(filters.sort).toBe('name');
  });
});
