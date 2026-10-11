// Startup sequence: load settings, then language and theme, before the first render,
// so the window never flashes the wrong theme or language.
import { createEffect, createRoot, on } from 'solid-js';
import { reportError, setLocale } from '../features/i18n';
import { flushLayout } from '../features/layout';
import { flushSettings, loadSettings, settings } from '../features/settings';
import { startThemeEngine } from '../features/theme';
import { startWorkspaceFs } from '../features/workspace';

async function applyLocale(code: typeof settings.language): Promise<void> {
  try {
    await setLocale(code);
  } catch (err) {
    reportError('error.localeLoad', err);
  }
}

export async function bootstrap(): Promise<void> {
  await loadSettings();
  // Language first, so any startup error toast (e.g. a missing theme) is already translated.
  await applyLocale(settings.language);
  await createRoot(() => {
    createEffect(
      on(
        () => settings.language,
        (code) => void applyLocale(code),
        { defer: true },
      ),
    );
    return startThemeEngine();
  });
  // Opens the workspace folder in the backend and keeps its watcher in sync.
  startWorkspaceFs();

  window.addEventListener('beforeunload', () => {
    flushLayout();
    void flushSettings();
  });
}
