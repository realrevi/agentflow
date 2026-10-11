import { describe, expect, it } from 'vitest';
import type { FsEntry } from '../../shared/types';
import { applyListing, emptyTree, foldersToRefresh, visibleRows } from './treeModel';

const dir = (path: string): FsEntry => ({
  name: path.split('/').pop() ?? path,
  path,
  isDir: true,
  isSymlink: false,
});
const file = (path: string): FsEntry => ({ ...dir(path), isDir: false });

function sampleTree() {
  const state = emptyTree(['src', 'src/ui']);
  applyListing(state, '', [dir('docs'), dir('src'), file('README.md')]);
  applyListing(state, 'src', [dir('src/ui'), file('src/main.ts')]);
  applyListing(state, 'src/ui', [file('src/ui/App.tsx')]);
  return state;
}

const paths = (state: ReturnType<typeof emptyTree>) =>
  visibleRows(state).map((row) => `${'  '.repeat(row.depth)}${row.entry.name}`);

describe('file tree model', () => {
  it('flattens expanded folders depth first', () => {
    expect(paths(sampleTree())).toEqual([
      'docs',
      'src',
      '  ui',
      '    App.tsx',
      '  main.ts',
      'README.md',
    ]);
  });

  it('keeps expanded folders when a watcher batch re-lists their parent', () => {
    const state = sampleTree();
    const changes = { paths: ['NOTES.md'], structural: ['NOTES.md'], rescan: false };
    expect(foldersToRefresh(state, changes)).toEqual(['']);
    applyListing(state, '', [dir('docs'), dir('src'), file('NOTES.md'), file('README.md')]);
    expect(paths(state)).toEqual([
      'docs',
      'src',
      '  ui',
      '    App.tsx',
      '  main.ts',
      'NOTES.md',
      'README.md',
    ]);
    expect(state.expanded.has('src/ui')).toBe(true);
  });

  it('forgets folders that disappeared, with everything below them', () => {
    const state = sampleTree();
    applyListing(state, '', [dir('docs'), file('README.md')]);
    expect(paths(state)).toEqual(['docs', 'README.md']);
    expect(state.expanded.size).toBe(0);
    expect(state.children.has('src/ui')).toBe(false);
  });

  it('only re-lists folders that are loaded', () => {
    const state = sampleTree();
    const changes = {
      paths: ['src/ui/New.tsx', 'docs/a.md', 'src/main.ts'],
      structural: ['src/ui/New.tsx', 'docs/a.md'],
      rescan: false,
    };
    expect(foldersToRefresh(state, changes)).toEqual(['src/ui']);
    expect(foldersToRefresh(state, { paths: [], structural: [], rescan: true }).sort()).toEqual([
      '',
      'src',
      'src/ui',
    ]);
  });
});
