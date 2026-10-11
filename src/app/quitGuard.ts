// Closing the window with unsaved editors asks first: Save all / Don't save / Cancel.
// Pending layout and settings changes are written before the window goes away.
import { dirtyEditorCount, saveAllEditors } from '../features/editor';
import { t } from '../features/i18n';
import { flushLayout } from '../features/layout';
import { flushSettings } from '../features/settings';
import { askChoice } from '../shared/dialog';
import { ipc } from '../shared/ipc';

async function mayClose(): Promise<boolean> {
  const dirty = dirtyEditorCount();
  if (dirty > 0) {
    const choice = await askChoice({
      title: t('app.quit.title'),
      message: t('app.quit.body', { count: dirty }),
      choices: [
        { id: 'save', label: t('app.quit.saveAll'), kind: 'primary' },
        { id: 'discard', label: t('editor.discard'), kind: 'danger' },
        { id: 'cancel', label: t('dialog.cancel') },
      ],
      cancelId: 'cancel',
    });
    if (choice === 'cancel') return false;
    // A failed save keeps the window open; the error toast says why.
    if (choice === 'save' && !(await saveAllEditors())) return false;
  }
  flushLayout();
  await flushSettings();
  return true;
}

/** Returns a cleanup function. Outside Tauri (tests, browser preview) it does nothing. */
export function installQuitGuard(): () => void {
  let unlisten: (() => void) | undefined;
  let disposed = false;
  ipc.onWindowClose(mayClose).then(
    (stop) => (disposed ? stop() : (unlisten = stop)),
    (err: unknown) => console.debug('[agentflow] no window close hook:', err),
  );
  return () => {
    disposed = true;
    unlisten?.();
  };
}
