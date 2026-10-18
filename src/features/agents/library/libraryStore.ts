// The community library's catalog in the Agents library panel: loaded from the backend
// cache (works offline), refreshed on demand, and automatically at most once a day (the
// backend decides) when the Browse tab first opens. The catalog is one plain object in a
// signal (a big listing; never a deep store).
import { createSignal } from 'solid-js';
import { ipc } from '../../../shared/ipc';
import type { LibraryCatalog } from '../../../shared/types';
import { reportError } from '../../i18n';
import { settings } from '../../settings/settingsStore';

const [catalog, setCatalog] = createSignal<LibraryCatalog | null>(null);
const [refreshing, setRefreshing] = createSignal(false);
let opened = false;

export { catalog, refreshing };

async function refresh(force: boolean): Promise<void> {
  if (refreshing()) return;
  setRefreshing(true);
  try {
    setCatalog(await ipc.libraryRefresh(force));
  } catch (err) {
    reportError('error.library', err);
  } finally {
    setRefreshing(false);
  }
}

/** First open of the Browse tab: the cached catalog, then the daily refresh if enabled. */
export async function openLibrary(): Promise<void> {
  if (opened) return;
  opened = true;
  try {
    setCatalog(await ipc.libraryCatalog());
  } catch (err) {
    reportError('error.library', err);
  }
  if (settings.library.autoRefresh) await refresh(false);
}

/** "Refresh" (and after the sources changed): asks GitHub now. */
export function refreshLibrary(): Promise<void> {
  return refresh(true);
}

/** The cached catalog for update checks in the Installed tab (no network). */
export async function ensureCatalog(): Promise<LibraryCatalog | null> {
  if (catalog() === null) {
    try {
      setCatalog(await ipc.libraryCatalog());
    } catch {
      return null;
    }
  }
  return catalog();
}
