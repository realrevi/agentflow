// The only place that talks to the Rust backend. Components and stores call `ipc.*`,
// never `invoke` directly. Each entry mirrors a #[tauri::command] in src-tauri/src/commands.
import { Channel, invoke } from '@tauri-apps/api/core';
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';
import { openUrl as openWithOs } from '@tauri-apps/plugin-opener';
import { AppError, toAppError } from './errors';
import type {
  AgentTemplate,
  Confirmations,
  GenerateRequest,
  InstallRequest,
} from './installerTypes';
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
  HandoffEntry,
  PtyMessage,
  PriceOverride,
  PtySpawnRequest,
  SessionInfo,
  Settings,
  TurnEvent,
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
 * Turn ends and live activity of agent CLI panels, read from their session logs. One
 * subscriber at a time (a new subscribe replaces the old channel).
 */
async function turnsSubscribe(onEvent: (event: TurnEvent) => void): Promise<{ detach(): void }> {
  const channel = new Channel<TurnEvent>(onEvent);
  const detach = () => {
    channel.onmessage = () => {};
  };
  try {
    await call('turns_subscribe', { onEvent: channel });
    return { detach };
  } catch (err) {
    detach();
    throw err;
  }
}

/** An OS notification (asks for permission once); false if it could not be shown. */
async function notify(title: string, body: string): Promise<boolean> {
  try {
    let granted = await isPermissionGranted();
    if (!granted) granted = (await requestPermission()) === 'granted';
    if (granted) sendNotification({ title, body });
    return granted;
  } catch (err) {
    console.debug('[agentflow] notification failed:', err);
    return false;
  }
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

const JSON_FILES = [{ name: 'JSON', extensions: ['json'] }];

/** Native file picker for one JSON file. Resolves `null` if cancelled. */
async function pickJsonFile(title: string): Promise<string | null> {
  try {
    const picked = await openDialog({ multiple: false, title, filters: JSON_FILES });
    return typeof picked === 'string' ? picked : null;
  } catch (err) {
    throw toAppError(err);
  }
}

/** Native "save as" for a JSON file. Resolves `null` if cancelled. */
async function pickJsonSavePath(title: string, defaultPath: string): Promise<string | null> {
  try {
    return await saveDialog({ title, defaultPath, filters: JSON_FILES });
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
  agentPermissionAnswer: (requestId: number, conversationId: string, answer: PermissionAnswer) =>
    call('agent_permission_answer', { requestId, conversationId, answer }),
  /** Turns of a chat that changed files, newest first. */
  agentCheckpoints: (workspace: string | null, conversationId: string) =>
    call('agent_checkpoints', { workspace, conversationId }),
  /** Undoes a turn's file changes (created files go to the OS trash). */
  agentCheckpointRestore: (workspace: string | null, conversationId: string, turn: string) =>
    call('agent_checkpoint_restore', { workspace, conversationId, turn }),
  // Agent installer (phase 8).
  agentsStarters: () => call('agents_starters', {}),
  /** Problems with a template (empty = valid). */
  agentsValidate: (template: unknown) => call('agents_validate', { template }),
  agentsTemplateRead: (path: string) => call('agents_template_read', { path }),
  agentsTemplateWrite: (path: string, template: AgentTemplate) =>
    call('agents_template_write', { path, template }),
  agentsDetect: (workspacePath: string | null) => call('agents_detect', { workspacePath }),
  /** Every file the install would write, with before/after and conflicts. Writes nothing. */
  agentsPlan: (workspacePath: string | null, request: InstallRequest) =>
    call('agents_plan', { workspacePath, request }),
  agentsInstall: (workspacePath: string | null, request: InstallRequest, confirm: Confirmations) =>
    call('agents_install', { workspacePath, request, confirm }),
  agentsInstalled: (workspacePath: string | null) => call('agents_installed', { workspacePath }),
  agentsUninstallPlan: (workspacePath: string | null, id: string) =>
    call('agents_uninstall_plan', { workspacePath, id }),
  agentsUninstall: (
    workspacePath: string | null,
    id: string,
    confirm: Confirmations,
    keep: string[],
  ) => call('agents_uninstall', { workspacePath, id, confirm, keep }),
  agentsProfiles: (workspacePath: string | null) => call('agents_profiles', { workspacePath }),
  /** A new API chat with an installed AgentFlow agent's role, tools and mode. */
  agentsProfileChat: (
    workspacePath: string | null,
    profileId: string,
    providerId: string,
    model: string,
  ) => call('agents_profile_chat', { workspacePath, profileId, providerId, model }),
  agentsGenerate: (request: GenerateRequest) => call('agents_generate', { request }),
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
  // Workstreams (phase 9).
  turnsSubscribe,
  workstreamLogAppend: (workspace: string | null, entry: HandoffEntry) =>
    call('workstream_log_append', { workspace, entry }),
  workstreamLogRead: (workspace: string | null, workstream: string) =>
    call('workstream_log_read', { workspace, workstream }),
  notify,
  onWindowClose,
  pickFolder,
  pickJsonFile,
  pickJsonSavePath,
  readClipboard: () => clipboard(readText),
  writeClipboard: (text: string) => clipboard(() => writeText(text)),
  openUrl,
};
