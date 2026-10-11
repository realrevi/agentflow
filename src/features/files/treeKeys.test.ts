import { describe, expect, it } from 'vitest';
import type { TreeRow } from './treeModel';
import { treeKeyAction } from './treeKeys';

const row = (path: string, depth: number, isDir = false, expanded = false): TreeRow => ({
  entry: { name: path, path, isDir, isSymlink: false },
  depth,
  expanded,
});
const rows = [row('src', 0, true, true), row('src/a.ts', 1), row('README.md', 0)];
const key = (name: string, extra: Partial<KeyboardEvent> = {}) => ({
  key: name,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  ...extra,
});

describe('tree keyboard navigation', () => {
  it('moves the selection', () => {
    expect(treeKeyAction(key('ArrowDown'), rows, -1, 10)).toEqual({ type: 'select', index: 0 });
    expect(treeKeyAction(key('ArrowDown'), rows, 2, 10)).toEqual({ type: 'select', index: 2 });
    expect(treeKeyAction(key('End'), rows, 0, 10)).toEqual({ type: 'select', index: 2 });
    expect(treeKeyAction(key('PageUp'), rows, 2, 10)).toEqual({ type: 'select', index: 0 });
  });

  it('expands, enters and leaves folders', () => {
    expect(treeKeyAction(key('ArrowRight'), rows, 0, 10)).toEqual({ type: 'select', index: 1 });
    expect(treeKeyAction(key('ArrowLeft'), rows, 1, 10)).toEqual({ type: 'select', index: 0 });
    expect(treeKeyAction(key('ArrowLeft'), rows, 0, 10)).toEqual({ type: 'collapse', index: 0 });
    const collapsed = [row('src', 0, true, false)];
    expect(treeKeyAction(key('ArrowRight'), collapsed, 0, 10)).toEqual({
      type: 'expand',
      index: 0,
    });
  });

  it('maps Enter, F2 and Delete', () => {
    expect(treeKeyAction(key('Enter'), rows, 1, 10)?.type).toBe('activate');
    expect(treeKeyAction(key('F2'), rows, 1, 10)?.type).toBe('rename');
    expect(treeKeyAction(key('Delete'), rows, 1, 10)?.type).toBe('delete');
    expect(treeKeyAction(key('Backspace', { metaKey: true }), rows, 1, 10)?.type).toBe('delete');
    expect(treeKeyAction(key('Backspace'), rows, 1, 10)).toBeNull();
  });
});
