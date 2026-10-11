// Reactive theme state: available themes, the active theme, and the effect that keeps
// the document's CSS variables in sync with the theme + user overrides.
import { createEffect, createRoot, createSignal, on } from 'solid-js';
import { ipc } from '../../shared/ipc';
import { showToast } from '../../shared/toast';
import type { ThemeData, ThemeSummary } from '../../shared/types';
import { reportError, t } from '../i18n';
import { settings, updateSettings } from '../settings/settingsStore';
import { applyThemeVars } from './engine';
import { computeThemeVars } from './overrides';

export const DEFAULT_THEME_ID = 'agentflow-dark';

const [themes, setThemes] = createSignal<readonly ThemeSummary[]>([]);
const [activeTheme, setActiveTheme] = createSignal<ThemeData | null>(null);
const cache = new Map<string, ThemeData>();

export { themes, activeTheme };

export async function refreshThemes(): Promise<void> {
  try {
    setThemes(await ipc.listThemes());
  } catch (err) {
    reportError('error.themeList', err);
  }
}

async function loadTheme(id: string): Promise<ThemeData> {
  const cached = cache.get(id);
  if (cached) return cached;
  const theme = await ipc.getTheme(id);
  cache.set(id, theme);
  return theme;
}

/** Loads the theme selected in settings, falling back to the default if it is gone. */
async function activate(id: string): Promise<void> {
  try {
    setActiveTheme(await loadTheme(id));
  } catch (err) {
    reportError('error.themeLoad', err);
    if (id !== DEFAULT_THEME_ID) updateSettings((s) => (s.appearance.themeId = DEFAULT_THEME_ID));
  }
}

export function selectTheme(id: string): void {
  updateSettings((s) => (s.appearance.themeId = id));
}

export async function importThemeFile(file: File): Promise<void> {
  try {
    const summary = await ipc.importTheme(file.name, await file.text());
    cache.delete(summary.id);
    await refreshThemes();
    // Re-importing the active theme does not change the setting, so reload it directly.
    if (settings.appearance.themeId === summary.id) await activate(summary.id);
    else selectTheme(summary.id);
    showToast({ kind: 'success', title: t('settings.theme.imported', { name: summary.name }) });
  } catch (err) {
    reportError('error.themeImport', err);
  }
}

/** Starts the reactive wiring. Call once after settings are loaded. */
export function startThemeEngine(): Promise<void> {
  return createRoot(() => {
    // Deferred: the initial theme is activated explicitly below, exactly once.
    createEffect(
      on(
        () => settings.appearance.themeId,
        (id) => void activate(id),
        { defer: true },
      ),
    );
    createEffect(() => {
      const theme = activeTheme();
      if (theme) applyThemeVars(computeThemeVars(theme, settings.appearance), theme.kind);
    });
    return Promise.all([refreshThemes(), activate(settings.appearance.themeId)]).then(() => {});
  });
}
