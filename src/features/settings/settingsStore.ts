// Frontend copy of the settings. The Rust side owns the file; every change here is
// applied instantly (reactive store) and persisted with a short debounce.
import { createStore, produce, unwrap } from 'solid-js/store';
import { debounce } from '../../shared/debounce';
import { ipc } from '../../shared/ipc';
import type { SerializedLayout, Settings } from '../../shared/types';
import { reportError } from '../i18n';
import { DEFAULT_SETTINGS } from './defaults';

/** Everything except the layout, which is large, opaque and not rendered reactively. */
export type ReactiveSettings = Omit<Settings, 'layout'>;

const SAVE_DELAY_MS = 300;

const { layout: defaultLayout, ...defaults } = structuredClone(DEFAULT_SETTINGS);
const [settings, setSettings] = createStore<ReactiveSettings>(defaults);
let layout: SerializedLayout | null = defaultLayout;

export { settings };

export async function loadSettings(): Promise<void> {
  try {
    const { layout: savedLayout, ...rest } = await ipc.getSettings();
    setSettings(rest);
    layout = savedLayout;
  } catch (err) {
    reportError('error.settingsLoad', err);
  }
}

/** Mutate settings in place, e.g. `updateSettings((s) => (s.language = 'de'))`. */
export function updateSettings(mutate: (draft: ReactiveSettings) => void): void {
  setSettings(produce(mutate));
  persist();
}

export function savedLayout(): SerializedLayout | null {
  return layout;
}

export function saveLayout(next: SerializedLayout | null): void {
  layout = next;
  persist();
}

/** Writes any pending change right away (used when the window is closing). */
export function flushSettings(): void {
  persist.flush();
}

const persist = debounce(() => void save(), SAVE_DELAY_MS);

async function save(): Promise<void> {
  try {
    await ipc.saveSettings({ ...structuredClone(unwrap(settings)), layout });
  } catch (err) {
    reportError('error.settingsSave', err);
  }
}
