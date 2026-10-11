// The only place that talks to the Rust backend. Components and stores call `ipc.*`,
// never `invoke` directly. Each entry mirrors a #[tauri::command] in src-tauri/src/commands.
import { Channel, invoke } from '@tauri-apps/api/core';
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { toAppError } from './errors';
import type {
  CliStatus,
  PtyMessage,
  PtySpawnRequest,
  SessionInfo,
  Settings,
  ThemeData,
  ThemeSummary,
} from './types';

interface Commands {
  get_settings: { args: Record<string, never>; result: Settings };
  save_settings: { args: { settings: Settings }; result: Settings };
  list_themes: { args: Record<string, never>; result: ThemeSummary[] };
  get_theme: { args: { id: string }; result: ThemeData };
  import_theme: { args: { fileName: string; contents: string }; result: ThemeSummary };
  pty_spawn: { args: PtySpawnRequest & { onOutput: Channel<PtyMessage> }; result: SessionInfo };
  pty_write: { args: { id: number; data: string }; result: null };
  pty_ack: { args: { id: number; bytes: number }; result: null };
  pty_resize: { args: { id: number; cols: number; rows: number }; result: null };
  pty_kill: { args: { id: number }; result: boolean };
  pty_list: { args: Record<string, never>; result: SessionInfo[] };
  cli_detect: { args: { refresh: boolean }; result: CliStatus[] };
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

/** A running terminal session. `detach()` stops delivering its output. */
export interface PtyHandle {
  info: SessionInfo;
  detach(): void;
}

/**
 * Starts a terminal. The channel is created before the call, so no output is missed.
 * Rust drops the channel when the session ends, which unregisters its JS callback.
 */
async function ptySpawn(
  request: PtySpawnRequest,
  onMessage: (message: PtyMessage) => void,
): Promise<PtyHandle> {
  const channel = new Channel<PtyMessage>(onMessage);
  const detach = () => {
    channel.onmessage = () => {};
  };
  try {
    const info = await call('pty_spawn', { ...request, onOutput: channel });
    return { info, detach };
  } catch (err) {
    detach();
    throw err;
  }
}

/** Native folder picker (tauri-plugin-dialog). Resolves `null` if cancelled. */
async function pickFolder(title: string): Promise<string | null> {
  try {
    const picked = await openDialog({ directory: true, multiple: false, title });
    return typeof picked === 'string' ? picked : null;
  } catch (err) {
    throw toAppError(err);
  }
}

/** Native clipboard (tauri-plugin-clipboard-manager): no webview permission prompts. */
async function clipboard<T>(run: () => Promise<T>): Promise<T> {
  try {
    return await run();
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
  ptySpawn,
  ptyWrite: (id: number, data: string) => call('pty_write', { id, data }),
  ptyAck: (id: number, bytes: number) => call('pty_ack', { id, bytes }),
  ptyResize: (id: number, cols: number, rows: number) => call('pty_resize', { id, cols, rows }),
  ptyKill: (id: number) => call('pty_kill', { id }),
  ptyList: () => call('pty_list', {}),
  cliDetect: (refresh: boolean) => call('cli_detect', { refresh }),
  pickFolder,
  readClipboard: () => clipboard(readText),
  writeClipboard: (text: string) => clipboard(() => writeText(text)),
};
