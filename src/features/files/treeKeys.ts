// Keyboard navigation for the file tree, as a pure function of the visible rows.
import type { TreeRow } from './treeModel';

export type TreeKeyAction = {
  type: 'select' | 'expand' | 'collapse' | 'activate' | 'rename' | 'delete';
  index: number;
} | null;

type KeyInfo = Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'>;

function parentIndex(rows: readonly TreeRow[], index: number): number {
  const depth = rows[index]?.depth ?? 0;
  for (let i = index - 1; i >= 0; i--) if ((rows[i]?.depth ?? 0) < depth) return i;
  return -1;
}

/** `index` is the selected row (-1 for none); `page` is how many rows fit on screen. */
export function treeKeyAction(
  event: KeyInfo,
  rows: readonly TreeRow[],
  index: number,
  page: number,
): TreeKeyAction {
  const last = rows.length - 1;
  if (last < 0 || event.altKey || event.ctrlKey) return null;
  const clamp = (i: number) => Math.max(0, Math.min(last, i));
  const row = rows[index];
  switch (event.key) {
    case 'ArrowDown':
      return { type: 'select', index: index < 0 ? 0 : clamp(index + 1) };
    case 'ArrowUp':
      return { type: 'select', index: index < 0 ? 0 : clamp(index - 1) };
    case 'Home':
      return { type: 'select', index: 0 };
    case 'End':
      return { type: 'select', index: last };
    case 'PageDown':
      return { type: 'select', index: clamp(index + page) };
    case 'PageUp':
      return { type: 'select', index: clamp(index - page) };
  }
  if (!row) return null;
  switch (event.key) {
    case 'ArrowRight':
      if (row.entry.isDir && !row.expanded) return { type: 'expand', index };
      if (row.expanded && (rows[index + 1]?.depth ?? 0) > row.depth) {
        return { type: 'select', index: index + 1 };
      }
      return null;
    case 'ArrowLeft': {
      if (row.expanded) return { type: 'collapse', index };
      const parent = parentIndex(rows, index);
      return parent < 0 ? null : { type: 'select', index: parent };
    }
    case 'Enter':
      return { type: 'activate', index };
    case 'F2':
      return { type: 'rename', index };
    case 'Delete':
      return { type: 'delete', index };
    case 'Backspace':
      return event.metaKey ? { type: 'delete', index } : null;
  }
  return null;
}
