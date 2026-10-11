// The only place that talks to the Rust backend. Components and stores call `ipc.*`,
// never `invoke` directly. Each entry mirrors a #[tauri::command] in src-tauri/src/commands.
import { invoke } from '@tauri-apps/api/core';
import { toAppError } from './errors';
import type { Settings, ThemeData, ThemeSummary } from './types';

interface Commands {
  get_settings: { args: Record<string, never>; result: Settings };
  save_settings: { args: { settings: Settings }; result: Settings };
  list_themes: { args: Record<string, never>; result: ThemeSummary[] };
  get_theme: { args: { id: string }; result: ThemeData };
  import_theme: { args: { fileName: string; contents: string }; result: ThemeSummary };
}

async function call<K extends keyof Commands>(
  command: K,
  args: Commands[K]['args'],
): Promise<Commands[K]['result']> {
  try {
    return await invoke<Commands[K]['result']>(command, args);
  } catch (err) {
    throw toAppError(err);
  }
}

export const ipc = {
  getSettings: () => call('get_settings', {}),
  saveSettings: (settings: Settings) => call('save_settings', { settings }),
  listThemes: () => call('list_themes', {}),
  getTheme: (id: string) => call('get_theme', { id }),
  importTheme: (fileName: string, contents: string) => call('import_theme', { fileName, contents }),
};
