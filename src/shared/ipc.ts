// The only place that talks to the Rust backend. Components and stores call `ipc.*`,
// never `invoke` directly. Each entry mirrors a #[tauri::command] in src-tauri/src/commands.
import { Channel, invoke } from '@tauri-apps/api/core';
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { openUrl as openWithOs } from '@tauri-apps/plugin-opener';
import { AppError, toAppError } from './errors';
import type { Commands } from './ipcCommands';
import { webUrl } from './url';
import type {
  ChatEvent,
  ChatSendRequest,
  CompactRequest,
  EstimateRequest,
  PermissionAnswer,
  ChatStarted,
  ConversationPatch,
  EntryKind,
  FsChanges,
  PtyMessage,
  PriceOverride,
  PtySpawnRequest,
  SessionInfo,
  Settings,
  UsageEvent,
} from './types';

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

/**
 * Opens the workspace in the backend and starts its watcher. Change batches arrive on
 * `onChanges` until `detach()` or the next open (Rust then drops the old channel).
 */
async function fsOpenWorkspace(
  root: string,
  showHidden: boolean,
  onChanges: (changes: FsChanges) => void,
): Promise<{ detach(): void }> {
  const channel = new Channel<FsChanges>(onChanges);
  const detach = () => {
    channel.onmessage = () => {};
  };
  try {
    await call('fs_open_workspace', { root, showHidden, onEvent: channel });
    return { detach };
  } catch (err) {
    detach();
    throw err;
  }
}

/**
 * Starts an API chat reply. The channel exists before the call, so no event is missed;
 * events can arrive before the promise resolves. Rust drops the channel after the final
 * stop/error event. `detach()` stops delivery (e.g. the panel closed).
 */
async function chatSend(
  request: ChatSendRequest,
  onEvent: (event: ChatEvent) => void,
): Promise<{ started: ChatStarted; detach(): void }> {
  const channel = new Channel<ChatEvent>(onEvent);
  const detach = () => {
    channel.onmessage = () => {};
  };
  try {
    const started = await call('chat_send', { request, onEvent: channel });
    return { started, detach };
  } catch (err) {
    detach();
    throw err;
  }
}

/** The local UTC offset in minutes east of UTC, as the usage commands expect it. */
function utcOffsetMinutes(): number {
  return -new Date().getTimezoneOffset();
}

/**
 * Live usage changes (new usage per panel, repricing, budget warnings). One subscriber at a
 * time: a new subscribe replaces the old channel. `detach()` stops delivery.
 */
async function usageSubscribe(onEvent: (event: UsageEvent) => void): Promise<{ detach(): void }> {
  const channel = new Channel<UsageEvent>(onEvent);
  const detach = () => {
    channel.onmessage = () => {};
  };
  try {
    await call('usage_subscribe', { offsetMinutes: utcOffsetMinutes(), onEvent: channel });
    return { detach };
  } catch (err) {
    detach();
    throw err;
  }
}

/**
 * Asks `guard` before the window closes; it resolves `false` to keep the window open. A guard
 * that throws lets the window close, so a bug can never trap the user. Returns an unlisten fn.
 */
async function onWindowClose(guard: () => Promise<boolean>): Promise<() => void> {
  return getCurrentWindow().onCloseRequested(async (event) => {
    let close = true;
    try {
      close = await guard();
    } catch (err) {
      console.error('[agentflow] close guard failed:', err);
    }
    if (!close) event.preventDefault();
  });
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

/** Opens an http(s) URL in the system browser (tauri-plugin-opener); other schemes are refused. */
async function openUrl(href: string): Promise<void> {
  const url = webUrl(href);
  if (!url) throw new AppError('invalidInput', href);
  try {
    await openWithOs(url);
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
  fsOpenWorkspace,
  fsCloseWorkspace: () => call('fs_close_workspace', {}),
  fsList: (dir: string) => call('fs_list', { dir }),
  fsRead: (path: string) => call('fs_read', { path }),
  /** `expectedMtime: null` overwrites unconditionally (e.g. "keep mine" after a delete). */
  fsWrite: (path: string, content: string, expectedMtime: number | null) =>
    call('fs_write', { path, content, expectedMtime }),
  fsCreate: (path: string, kind: EntryKind) => call('fs_create', { path, kind }),
  fsRename: (from: string, to: string) => call('fs_rename', { from, to }),
  fsDelete: (path: string) => call('fs_delete', { path }),
  fsQuickOpen: (query: string) => call('fs_quick_open', { query }),
  promptsList: (workspace: string | null, panel: string) =>
    call('prompts_list', { workspace, panel }),
  promptsAdd: (workspace: string | null, panel: string, text: string) =>
    call('prompts_add', { workspace, panel, text }),
  conversationsList: (workspace: string | null) => call('conversations_list', { workspace }),
  conversationGet: (workspace: string | null, id: string) =>
    call('conversation_get', { workspace, id }),
  conversationCreate: (
    workspace: string | null,
    providerId: string | null,
    model: string,
    systemPrompt = '',
  ) => call('conversation_create', { workspace, providerId, model, systemPrompt }),
  conversationUpdate: (workspace: string | null, id: string, patch: ConversationPatch) =>
    call('conversation_update', { workspace, id, patch }),
  conversationDelete: (workspace: string | null, id: string) =>
    call('conversation_delete', { workspace, id }),
  chatSend,
  chatCancel: (requestId: number) => call('chat_cancel', { requestId }),
  /** Summarizes the older turns (the last `chat.compactKeepTurns` stay verbatim). */
  chatCompact: (request: CompactRequest) => call('chat_compact', { request }),
  /** The answer to an agent's permission request; `false` if the turn already ended. */
  agentPermissionAnswer: (requestId: number, answer: PermissionAnswer) =>
    call('agent_permission_answer', { requestId, answer }),
  /** Turns of a chat that changed files, newest first. */
  agentCheckpoints: (workspace: string | null, conversationId: string) =>
    call('agent_checkpoints', { workspace, conversationId }),
  /** Undoes a turn's file changes (created files go to the OS trash). */
  agentCheckpointRestore: (workspace: string | null, conversationId: string, turn: string) =>
    call('agent_checkpoint_restore', { workspace, conversationId, turn }),
  /** Keys are write-only: the frontend only ever sees has-key + the last 4 characters. */
  providerKeyStatus: (providerId: string) => call('provider_key_status', { providerId }),
  providerSetKey: (providerId: string, key: string) =>
    call('provider_set_key', { providerId, key }),
  providerDeleteKey: (providerId: string) => call('provider_delete_key', { providerId }),
  providerModels: (providerId: string) => call('provider_models', { providerId }),
  providerTest: (providerId: string) => call('provider_test', { providerId }),
  usageSubscribe,
  usageSummary: () => call('usage_summary', { offsetMinutes: utcOffsetMinutes() }),
  /** One panel's totals since `since` (ms). */
  usagePanelTotals: (panel: string, since: number) => call('usage_panel_totals', { panel, since }),
  usageEstimate: (request: EstimateRequest) => call('usage_estimate', { request }),
  usageBudgets: () => call('usage_budgets', { offsetMinutes: utcOffsetMinutes() }),
  /** A single-use token for `chatSend` (`allowOnce`) past a hard budget stop. */
  usageAllowOnce: (providerId: string) => call('usage_allow_once', { providerId }),
  usagePrices: () => call('usage_prices', {}),
  /** Saves price overrides; use `setPriceOverrides()` from the settings store. */
  usageSetPrices: (prices: PriceOverride[]) => call('usage_set_prices', { prices }),
  usageCliStatus: () => call('usage_cli_status', {}),
  onWindowClose,
  pickFolder,
  readClipboard: () => clipboard(readText),
  writeClipboard: (text: string) => clipboard(() => writeText(text)),
  openUrl,
};
