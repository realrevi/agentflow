// Startup sequence: load settings, then language and theme, before the first render,
// so the window never flashes the wrong theme or language.
import { createEffect, createRoot, on } from 'solid-js';
import { installAttentionStyle, startTurnFeed } from '../features/agentui';
import { reportError, setLocale } from '../features/i18n';
import { flushLayout } from '../features/layout';
import { flushSettings, loadSettings, settings } from '../features/settings';
import { startThemeEngine } from '../features/theme';
import { startUsage } from '../features/tokens';
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
  // One usage subscription for the status bar, badges and budget alerts (not awaited:
  // the first render never waits for usage).
  void startUsage();
  // Turn ends and live activity of agent CLI panels (one Channel), and the glow color.
  void startTurnFeed();
  installAttentionStyle();

  window.addEventListener('beforeunload', () => {
    flushLayout();
    void flushSettings();
  });
}
