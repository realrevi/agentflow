// The typed map of every backend command (see ipc.ts). Each entry mirrors a
// #[tauri::command] in src-tauri/src/commands.
import type { Channel } from '@tauri-apps/api/core';
import type {
  AllowOnceGrant,
  CheckpointSummary,
  PermissionAnswer,
  RestoreReport,
  BudgetStatus,
  ChatEvent,
  ChatSendRequest,
  ChatStarted,
  CliStatus,
  CliSupport,
  CompactRequest,
  CompactResult,
  ConnectionTest,
  Conversation,
  ConversationPatch,
  ConversationSummary,
  EntryKind,
  Estimate,
  EstimateRequest,
  FileContent,
  FileStat,
  FsChanges,
  FsEntry,
  KeyStatus,
  ModelInfo,
  PriceList,
  PriceOverride,
  PromptEntry,
  PtyMessage,
  PtySpawnRequest,
  QuickOpenMatch,
  SessionInfo,
  Settings,
  ThemeData,
  ThemeSummary,
  Totals,
  UsageEvent,
  UsageSummary,
} from './types';

type Workspace = { workspace: string | null };
type ProviderId = { providerId: string };
/** Minutes east of UTC (`-new Date().getTimezoneOffset()`): local days and months. */
type Offset = { offsetMinutes: number };

export interface Commands {
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
  fs_open_workspace: {
    args: { root: string; showHidden: boolean; onEvent: Channel<FsChanges> };
    result: null;
  };
  fs_close_workspace: { args: Record<string, never>; result: null };
  fs_list: { args: { dir: string }; result: FsEntry[] };
  fs_read: { args: { path: string }; result: FileContent };
  fs_write: {
    args: { path: string; content: string; expectedMtime: number | null };
    result: FileStat;
  };
  fs_create: { args: { path: string; kind: EntryKind }; result: FsEntry };
  fs_rename: { args: { from: string; to: string }; result: FsEntry };
  fs_delete: { args: { path: string }; result: null };
  fs_quick_open: { args: { query: string }; result: QuickOpenMatch[] };
  prompts_list: { args: { workspace: string | null; panel: string }; result: PromptEntry[] };
  prompts_add: {
    args: { workspace: string | null; panel: string; text: string };
    result: PromptEntry;
  };
  conversations_list: { args: Workspace; result: ConversationSummary[] };
  conversation_get: { args: Workspace & { id: string }; result: Conversation };
  conversation_create: {
    args: Workspace & { providerId: string | null; model: string; systemPrompt: string };
    result: Conversation;
  };
  conversation_update: {
    args: Workspace & { id: string; patch: ConversationPatch };
    result: ConversationSummary;
  };
  conversation_delete: { args: Workspace & { id: string }; result: null };
  chat_send: {
    args: { request: ChatSendRequest; onEvent: Channel<ChatEvent> };
    result: ChatStarted;
  };
  chat_cancel: { args: { requestId: number }; result: boolean };
  chat_compact: { args: { request: CompactRequest }; result: CompactResult };
  agent_permission_answer: {
    args: { requestId: number; conversationId: string; answer: PermissionAnswer };
    result: boolean;
  };
  agent_checkpoints: { args: Workspace & { conversationId: string }; result: CheckpointSummary[] };
  agent_checkpoint_restore: {
    args: Workspace & { conversationId: string; turn: string };
    result: RestoreReport;
  };
  provider_key_status: { args: ProviderId; result: KeyStatus };
  provider_set_key: { args: ProviderId & { key: string }; result: KeyStatus };
  provider_delete_key: { args: ProviderId; result: null };
  provider_models: { args: ProviderId; result: ModelInfo[] };
  provider_test: { args: ProviderId; result: ConnectionTest };
  usage_subscribe: { args: Offset & { onEvent: Channel<UsageEvent> }; result: null };
  usage_summary: { args: Offset; result: UsageSummary };
  usage_panel_totals: { args: { panel: string; since: number }; result: Totals };
  usage_estimate: { args: { request: EstimateRequest }; result: Estimate };
  usage_budgets: { args: Offset; result: BudgetStatus[] };
  usage_allow_once: { args: ProviderId; result: AllowOnceGrant };
  usage_prices: { args: Record<string, never>; result: PriceList };
  usage_set_prices: { args: { prices: PriceOverride[] }; result: Settings };
  usage_cli_status: { args: Record<string, never>; result: CliSupport[] };
}
