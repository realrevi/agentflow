// App updates (phase 12): a check on launch (setting) or from Settings > About. A newer
// version shows ONE sticky toast with its notes; only "Install and restart" downloads it.
// Rust verifies the signature against the public key in tauri.conf.json before installing.
import { createSignal } from 'solid-js';
import { ipc } from '../../shared/ipc';
import { showToast } from '../../shared/toast';
import type { UpdateInfo } from '../../shared/types';
import { reportError, t } from '../i18n';

export type UpdateStatus = 'idle' | 'checking' | 'upToDate' | 'available' | 'installing' | 'error';

const [status, setStatus] = createSignal<UpdateStatus>('idle');
const [available, setAvailable] = createSignal<UpdateInfo | null>(null);
/** Download share 0..1 while installing; null when the size is unknown. */
const [progress, setProgress] = createSignal<number | null>(null);

export { available as availableUpdate, progress as updateProgress, status as updateStatus };

/** Wait after launch, so the check never competes with startup work. */
export const STARTUP_CHECK_DELAY_MS = 8000;
const NOTES_MAX = 160;

/** The first lines of the release notes, short enough for a toast (markdown marks dropped). */
export function shortNotes(notes: string | null, max = NOTES_MAX): string | undefined {
  if (!notes) return undefined;
  const text = notes
    .split(/\r?\n/)
    .map((line) => line.replace(/^#+\s*|^[-*]\s+|\*\*/g, '').trim())
    .filter(Boolean)
    .join(' · ');
  if (!text) return undefined;
  return text.length > max ? `${text.slice(0, max - 1).trimEnd()}…` : text;
}

function offer(info: UpdateInfo): void {
  showToast({
    kind: 'info',
    title: t('updates.available', { version: info.version }),
    detail: shortNotes(info.notes),
    sticky: true,
    action: { label: t('updates.install'), run: () => void installUpdate() },
  });
}

/**
 * Looks for a newer release. `manual` (Settings > About) also reports "up to date" and
 * errors; the launch check stays silent unless there is something to install.
 */
export async function checkForUpdates(manual: boolean): Promise<void> {
  if (status() === 'checking' || status() === 'installing') return;
  setStatus('checking');
  try {
    const found = await ipc.updateCheck();
    setAvailable(found);
    setStatus(found ? 'available' : 'upToDate');
    if (found) offer(found);
    else if (manual) {
      showToast({ kind: 'success', title: t('updates.upToDate', { version: __APP_VERSION__ }) });
    }
  } catch (err) {
    setStatus('error');
    if (manual) reportError('updates.checkFailed', err);
    else console.warn('[agentflow] update check failed', err);
  }
}

/** Downloads, verifies and installs the update found last, then the app restarts. */
export async function installUpdate(): Promise<void> {
  if (status() === 'installing' || !available()) return;
  setStatus('installing');
  setProgress(null);
  let total: number | null = null;
  try {
    await ipc.updateInstall((event) => {
      if (event.type === 'started') total = event.total;
      else if (event.type === 'progress' && total) setProgress(event.downloaded / total);
      else if (event.type === 'finished') setProgress(1);
    });
  } catch (err) {
    setStatus('available');
    reportError('updates.installFailed', err);
  }
}

/**
 * The launch check (release builds, if the setting is on), a few seconds after startup.
 * Returns a function that cancels it.
 */
export function scheduleStartupCheck(enabled: () => boolean): () => void {
  if (import.meta.env.DEV) return () => {};
  const timer = window.setTimeout(() => {
    if (enabled()) void checkForUpdates(false);
  }, STARTUP_CHECK_DELAY_MS);
  return () => window.clearTimeout(timer);
}
