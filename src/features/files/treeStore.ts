// Reactive wrapper around the tree model for one Files panel: loads listings lazily,
// applies watcher batches and remembers expanded folders per workspace for the session.
import { createMemo, createSignal } from 'solid-js';
import { toAppError } from '../../shared/errors';
import { ipc } from '../../shared/ipc';
import type { FsChanges } from '../../shared/types';
import { reportError } from '../i18n';
import { parentPath } from '../workspace';
import {
  applyListing,
  emptyTree,
  foldersToRefresh,
  forget,
  visibleRows,
  type TreeRow,
} from './treeModel';

/** Expanded folders per workspace, kept while the app runs (survives closing the panel). */
const expandedMemory = new Map<string, Set<string>>();

export interface TreeStore {
  rows: () => readonly TreeRow[];
  reset(root: string | null): void;
  toggle(path: string): void;
  expand(path: string): Promise<void>;
  collapse(path: string): void;
  collapseAll(): void;
  /** Re-lists every loaded folder. */
  refresh(): void;
  /** Re-lists one folder now (after a create/rename/delete from the tree). */
  reload(dir: string): Promise<void>;
  /** Re-lists the parent of `path` and expands every folder above it, so it is shown. */
  reveal(path: string): Promise<void>;
  applyChanges(changes: FsChanges): void;
}

export function createTreeStore(): TreeStore {
  let state = emptyTree();
  let generation = 0;
  /** Latest request per folder, so a slow older listing never overwrites a newer one. */
  const requests = new Map<string, number>();
  let nextRequest = 0;
  const [version, setVersion] = createSignal(0);
  const changed = () => setVersion((value) => value + 1);
  const rows = createMemo(() => {
    version();
    return visibleRows(state);
  });

  async function load(dir: string): Promise<void> {
    const ticket = ++nextRequest;
    const startedIn = generation;
    requests.set(dir, ticket);
    try {
      const entries = await ipc.fsList(dir);
      if (startedIn !== generation || requests.get(dir) !== ticket) return;
      applyListing(state, dir, entries);
    } catch (err) {
      if (startedIn !== generation || requests.get(dir) !== ticket) return;
      // A folder removed in the meantime simply disappears from the tree.
      if (dir && toAppError(err).kind === 'notFound') forget(state, dir);
      else reportError('error.fsList', err);
    }
    changed();
  }

  async function expand(path: string): Promise<void> {
    state.expanded.add(path);
    changed();
    if (!state.children.has(path)) await load(path);
  }

  function collapse(path: string): void {
    state.expanded.delete(path);
    changed();
  }

  return {
    rows,
    expand,
    collapse,
    reset(root) {
      generation++;
      requests.clear();
      const remembered = root ? (expandedMemory.get(root) ?? new Set<string>()) : new Set<string>();
      if (root) expandedMemory.set(root, remembered);
      state = { children: new Map(), expanded: remembered };
      changed();
      if (!root) return;
      void load('');
      for (const dir of remembered) void load(dir);
    },
    toggle(path) {
      if (state.expanded.has(path)) collapse(path);
      else void expand(path);
    },
    collapseAll() {
      state.expanded.clear();
      changed();
    },
    refresh() {
      for (const dir of [...state.children.keys()]) void load(dir);
    },
    reload(dir) {
      return load(dir);
    },
    async reveal(path) {
      const parent = parentPath(path);
      await load(parent);
      const ancestors: string[] = [];
      for (let dir = parent; dir; dir = parentPath(dir)) ancestors.unshift(dir);
      for (const dir of ancestors) await expand(dir);
    },
    applyChanges(changes) {
      for (const dir of foldersToRefresh(state, changes)) void load(dir);
    },
  };
}

/** The folder a new entry goes into when `path` is selected (a file's parent). */
export function targetFolder(row: TreeRow | undefined): string {
  if (!row) return '';
  return row.entry.isDir ? row.entry.path : parentPath(row.entry.path);
}
