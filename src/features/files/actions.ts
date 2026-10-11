// File operations started from the tree: create, rename, delete (to trash), copy paths.
import { askChoice } from '../../shared/dialog';
import { ipc } from '../../shared/ipc';
import { showToast } from '../../shared/toast';
import type { EntryKind } from '../../shared/types';
import { reportError, t } from '../i18n';
import { absolutePath, baseName, currentWorkspace, joinPath, parentPath } from '../workspace';
import type { TreeStore } from './treeStore';

export interface FileCallbacks {
  onOpenFile(path: string): void;
  onRenamed(from: string, to: string): void;
}

/** Creates `name` (may contain `/`) in `parent`. Returns the new path, or null. */
export async function createEntry(
  tree: TreeStore,
  parent: string,
  name: string,
  kind: EntryKind,
  callbacks: FileCallbacks,
): Promise<string | null> {
  try {
    const entry = await ipc.fsCreate(joinPath(parent, name), kind);
    await tree.reveal(entry.path);
    if (kind === 'file') callbacks.onOpenFile(entry.path);
    return entry.path;
  } catch (err) {
    reportError('error.fsCreate', err);
    return null;
  }
}

/** Renames `path` to `name` in the same folder. Returns the new path, or null. */
export async function renameEntry(
  tree: TreeStore,
  path: string,
  name: string,
  callbacks: FileCallbacks,
): Promise<string | null> {
  if (name === baseName(path)) return path;
  try {
    const entry = await ipc.fsRename(path, joinPath(parentPath(path), name));
    await tree.reveal(entry.path);
    callbacks.onRenamed(path, entry.path);
    return entry.path;
  } catch (err) {
    reportError('error.fsRename', err);
    return null;
  }
}

/** Asks, then moves `path` to the OS trash. Resolves true if it was deleted. */
export async function deleteEntry(tree: TreeStore, path: string, isDir: boolean): Promise<boolean> {
  const choice = await askChoice({
    title: t(isDir ? 'files.delete.folderTitle' : 'files.delete.fileTitle', {
      name: baseName(path),
    }),
    message: t('files.delete.body'),
    choices: [
      { id: 'delete', label: t('files.delete.confirm'), kind: 'danger' },
      { id: 'cancel', label: t('dialog.cancel') },
    ],
    cancelId: 'cancel',
  });
  if (choice !== 'delete') return false;
  try {
    await ipc.fsDelete(path);
    await tree.reload(parentPath(path));
    return true;
  } catch (err) {
    reportError('error.fsDelete', err);
    return false;
  }
}

export async function copyPath(path: string, absolute: boolean): Promise<void> {
  const root = currentWorkspace();
  const text = absolute && root ? absolutePath(root, path) : path;
  try {
    await ipc.writeClipboard(text);
    showToast({ kind: 'success', title: t('files.copied'), detail: text });
  } catch (err) {
    reportError('error.clipboard', err);
  }
}
