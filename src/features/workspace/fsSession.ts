// Connects the backend to the workspace folder: (re)opens it when the folder or the
// hidden-files setting changes, and fans the watcher's change batches out to listeners
// (file tree, editors). Opens run one at a time, so the newest settings always win.
import { createEffect, createRoot, createSignal, on } from 'solid-js';
import { ipc } from '../../shared/ipc';
import type { FsChanges } from '../../shared/types';
import { reportError } from '../i18n';
import { settings } from '../settings/settingsStore';

export type FsStatus = 'none' | 'opening' | 'ready' | 'error';
type Listener = (changes: FsChanges) => void;

const listeners = new Set<Listener>();
const [fsStatus, setFsStatus] = createSignal<FsStatus>('none');
/** The folder the backend serves right now (`null` while switching or without one). */
const [fsRoot, setFsRoot] = createSignal<string | null>(null);

export { fsRoot, fsStatus };

let queue: Promise<void> = Promise.resolve();
let detach: (() => void) | undefined;
let connectedHidden = false;
/** Watcher batches for the current workspace. Returns an unsubscribe function. */
export function onFsChanges(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Starts following the workspace setting. Returns a cleanup function. */
export function startWorkspaceFs(): () => void {
  return createRoot((dispose) => {
    createEffect(
      on(
        () => [settings.currentWorkspace, settings.files.showHidden] as const,
        () => {
          queue = queue.then(connect);
        },
      ),
    );
    return dispose;
  });
}

async function connect(): Promise<void> {
  // Read the settings now: a burst of changes collapses into the latest one.
  const root = settings.currentWorkspace;
  const showHidden = settings.files.showHidden;
  if (root === fsRoot() && showHidden === connectedHidden) return;
  detach?.();
  detach = undefined;
  setFsRoot(null);
  if (!root) {
    setFsStatus('none');
    await ipc.fsCloseWorkspace().catch((err: unknown) => console.debug(err));
    return;
  }
  setFsStatus('opening');
  try {
    const handle = await ipc.fsOpenWorkspace(root, showHidden, (changes) => {
      for (const listener of listeners) listener(changes);
    });
    detach = handle.detach;
    connectedHidden = showHidden;
    setFsRoot(root);
    setFsStatus('ready');
  } catch (err) {
    setFsStatus('error');
    reportError('error.workspaceOpen', err);
  }
}
