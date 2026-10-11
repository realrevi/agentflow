// The file tree's data, kept outside Solid so it is cheap at 10k+ entries and easy to
// test: folder listings by path, the expanded set, and the flat list of visible rows.
import type { FsChanges, FsEntry } from '../../shared/types';
import { isInside, parentPath } from '../workspace';

export interface TreeState {
  /** Loaded listings by folder path ('' is the workspace root). */
  children: Map<string, readonly FsEntry[]>;
  expanded: Set<string>;
}

export interface TreeRow {
  entry: FsEntry;
  depth: number;
  expanded: boolean;
}

export function emptyTree(expanded: Iterable<string> = []): TreeState {
  return { children: new Map(), expanded: new Set(expanded) };
}

/** Depth-first rows of every loaded entry whose ancestors are all expanded. */
export function visibleRows(state: TreeState): TreeRow[] {
  const rows: TreeRow[] = [];
  const walk = (dir: string, depth: number) => {
    for (const entry of state.children.get(dir) ?? []) {
      const expanded = entry.isDir && state.expanded.has(entry.path);
      rows.push({ entry, depth, expanded });
      if (expanded) walk(entry.path, depth + 1);
    }
  };
  walk('', 0);
  return rows;
}

/**
 * Stores a fresh listing of `dir`. Sub-folders that are gone are forgotten together with
 * everything loaded or expanded below them; everything else keeps its state.
 */
export function applyListing(state: TreeState, dir: string, entries: readonly FsEntry[]): void {
  state.children.set(dir, entries);
  const folders = new Set(entries.filter((entry) => entry.isDir).map((entry) => entry.path));
  const isGone = (path: string) => path !== dir && parentPath(path) === dir && !folders.has(path);
  const gone = [...state.children.keys(), ...state.expanded].filter(isGone);
  for (const folder of gone) forget(state, folder);
}

/** Drops a folder and everything below it (listings and expanded flags). */
export function forget(state: TreeState, folder: string): void {
  for (const path of [...state.children.keys()]) {
    if (isInside(path, folder)) state.children.delete(path);
  }
  for (const path of [...state.expanded]) {
    if (isInside(path, folder)) state.expanded.delete(path);
  }
}

/** Loaded folders to re-list for one watcher batch. */
export function foldersToRefresh(state: TreeState, changes: FsChanges): string[] {
  if (changes.rescan) return [...state.children.keys()];
  const folders = new Set<string>();
  for (const path of changes.structural) {
    const parent = parentPath(path);
    if (state.children.has(parent)) folders.add(parent);
    if (state.children.has(path)) folders.add(path);
  }
  return [...folders];
}
