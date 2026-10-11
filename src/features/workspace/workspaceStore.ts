// The workspace folder: where new terminals start. Stored in settings (Rust-owned).
import { ipc } from '../../shared/ipc';
import { reportError, t } from '../i18n';
import { settings, updateSettings } from '../settings/settingsStore';
import { addRecent } from './recent';

export const currentWorkspace = (): string | null => settings.currentWorkspace;
export const recentWorkspaces = (): readonly string[] => settings.recentWorkspaces;

export function setWorkspace(path: string): void {
  updateSettings((s) => {
    s.currentWorkspace = path;
    s.recentWorkspaces = addRecent(s.recentWorkspaces, path);
  });
}

/** Shows the native folder picker and switches to the chosen folder. */
export async function openFolder(): Promise<void> {
  try {
    const path = await ipc.pickFolder(t('workspace.pickTitle'));
    if (path) setWorkspace(path);
  } catch (err) {
    reportError('error.folderPick', err);
  }
}
