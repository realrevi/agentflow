// What the rest of the app needs from open editors: dirty flags (the tab dot) and a way
// to save before closing. The lazy EditorPanel registers each editor here.
import { createStore, produce } from 'solid-js/store';
import { askChoice } from '../../shared/dialog';
import { t } from '../i18n';

export interface EditorHandle {
  fileName(): string;
  /** Resolves `true` once saved; `false` if saving failed or was not possible. */
  save(): Promise<boolean>;
}

const [dirty, setDirty] = createStore<Record<string, boolean>>({});
const handles = new Map<string, EditorHandle>();

export function editorDirty(panelId: string): boolean {
  return dirty[panelId] === true;
}

export function dirtyEditorCount(): number {
  return Object.values(dirty).filter(Boolean).length;
}

export function setEditorDirty(panelId: string, value: boolean): void {
  setDirty(panelId, value);
}

/** Returns the unregister function (call it when the editor is disposed). */
export function registerEditor(panelId: string, handle: EditorHandle): () => void {
  handles.set(panelId, handle);
  return () => {
    if (handles.get(panelId) !== handle) return;
    handles.delete(panelId);
    setDirty(
      produce((all) => {
        delete all[panelId];
      }),
    );
  };
}

/** Save / Don't save / Cancel for a dirty editor. Resolves `true` when it may close. */
export async function confirmEditorClose(panelId: string): Promise<boolean> {
  const handle = handles.get(panelId);
  if (!handle || !editorDirty(panelId)) return true;
  const choice = await askChoice({
    title: t('editor.closeDirty.title', { name: handle.fileName() }),
    message: t('editor.closeDirty.body'),
    choices: [
      { id: 'save', label: t('editor.save'), kind: 'primary' },
      { id: 'discard', label: t('editor.discard'), kind: 'danger' },
      { id: 'cancel', label: t('dialog.cancel') },
    ],
    cancelId: 'cancel',
  });
  if (choice === 'save') return handle.save();
  return choice === 'discard';
}
