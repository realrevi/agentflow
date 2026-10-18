// AI providers, API chats and conversations (phase 5). Mirrors src-tauri/src/{settings,
// providers,conversations}. API keys never cross IPC: the frontend only sees KeyStatus.

import type {
  AgentPhase,
  AgentState,
  Part,
  PermissionMode,
  PermissionRequest,
  ToolCall,
  ToolResult,
} from './agentTypes';

export type ProviderKind = 'anthropic' | 'openai';
export type OpenAiPreset =
  'openai' | 'deepseek' | 'gemini' | 'openrouter' | 'groq' | 'ollama' | 'custom';
export type ProviderPreset = 'anthropic' | OpenAiPreset;

export interface ProviderEntry {
  id: string;
  kind: ProviderKind;
  preset: ProviderPreset;
  /** Empty = the preset's name. */
  name: string;
  baseUrl: string;
  defaultModel: string;
  maxTokens: number;
  /** `null` = the provider's default. */
  temperature: number | null;
}

export interface ModelWindow {
  model: string;
  tokens: number;
}

export interface ChatSettings {
  /** Streamed text is revealed smoothly; off = shown as it arrives. */
  smoothReveal: boolean;
  /** Anthropic: cache breakpoints on the system prompt, attachments and the stable prefix. */
  promptCaching: boolean;
  /** Used when neither the models endpoint nor `contextWindows` knows the model. */
  defaultContextWindow: number;
  contextWindows: ModelWindow[];
  /** Share of the context window at which the prompt bar warns (and auto-compact runs). */
  compactThreshold: number;
  /** Compact keeps this many recent turns verbatim. */
  compactKeepTurns: number;
  /** `null` = the chat's own provider. */
  compactProviderId: string | null;
  /** Empty = the chat's own model. */
  compactModel: string;
  autoCompact: boolean;
}

/** All the frontend ever learns about a stored API key. */
export interface KeyStatus {
  hasKey: boolean;
  last4: string | null;
}

export interface ModelInfo {
  id: string;
  name: string | null;
  /** Max input tokens, when the models endpoint says. */
  contextWindow: number | null;
}

export interface ConnectionTest {
  /** How many models the provider listed (0 when it was tested with a tiny request). */
  models: number;
  /** What was checked, e.g. "models" or "request". */
  method: 'models' | 'request';
}

export interface TokenUsage {
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheWriteTokens: number;
}

/** Stored with every assistant message so phase 6 can aggregate costs. */
export interface UsageRecord extends TokenUsage {
  providerId: string;
  preset: ProviderPreset;
  model: string;
  at: number;
}

/** A workspace file sent with a prompt (fenced, with its relative path). */
export interface Attachment {
  path: string;
  content: string;
}

/** `tool`: the results of an agent step (sent to the model as the user's turn). */
export type MessageRole = 'user' | 'assistant' | 'tool';
export type MessageStatus = 'complete' | 'cancelled' | 'error';

export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  attachments: Attachment[];
  at: number;
  status: MessageStatus;
  stopReason: string | null;
  error: string | null;
  usage: UsageRecord | null;
  /** Agent messages: reasoning, text and tool calls, or tool results. */
  parts?: Part[];
  /** Digest of the system prompt and tools an agent message was produced under. */
  prefix?: string;
}

export interface ConversationSummary {
  id: string;
  title: string;
  providerId: string | null;
  model: string;
  createdAt: number;
  updatedAt: number;
  messageCount: number;
}

export interface Conversation {
  version: number;
  id: string;
  title: string;
  workspace: string | null;
  providerId: string | null;
  model: string;
  systemPrompt: string;
  createdAt: number;
  updatedAt: number;
  messages: ChatMessage[];
  /** Set by Compact: requests send the summary instead of the messages up to `through`. */
  compaction: Compaction | null;
  agent: AgentState;
}

export interface Compaction {
  summary: string;
  /** Id of the last message the summary covers. */
  through: string;
  at: number;
  providerId: string;
  model: string;
}

export interface ConversationPatch {
  title?: string;
  providerId?: string | null;
  model?: string;
  systemPrompt?: string;
  permissionMode?: PermissionMode;
}

export type ChatErrorKind =
  | 'auth'
  | 'rateLimit'
  | 'invalidModel'
  | 'badRequest'
  | 'overloaded'
  | 'server'
  | 'network'
  | 'timeout'
  | 'config';

export interface ChatError {
  kind: ChatErrorKind;
  /** Already redacted by the backend. */
  message: string;
  status: number | null;
  /** Seconds to wait before retrying (429), when the provider said. */
  retryAfter: number | null;
}

/** Events of one turn, in order: deltas, usage and agent steps, then exactly one stop or
 * error. */
export type ChatEvent =
  | { type: 'delta'; text: string }
  | { type: 'reasoning'; text: string }
  | { type: 'usage'; usage: TokenUsage }
  | { type: 'status'; phase: AgentPhase }
  | { type: 'toolStart'; call: ToolCall }
  | { type: 'toolEnd'; result: ToolResult }
  | { type: 'permission'; request: PermissionRequest }
  | { type: 'notice'; kind: 'toolsUnsupported' | 'maxIterations'; message: string }
  | { type: 'message'; message: ChatMessage }
  | { type: 'stop'; reason: string; message: ChatMessage | null }
  | { type: 'error'; error: ChatError; message: ChatMessage | null };

export interface ChatSendRequest {
  workspace: string | null;
  conversationId: string;
  /** `null` = retry: resend the last user message (a failed reply after it is dropped). */
  text: string | null;
  attachments: Attachment[];
  /** The chat panel (per-panel usage). */
  panel: string | null;
  /** A token from `usage_allow_once`: passes a hard budget stop once. */
  allowOnce: string | null;
  /** Appended to the system prompt for this turn (workstream manager), never stored. */
  extraSystem: string | null;
}

export interface ChatStarted {
  requestId: number;
  /** The stored user message (null on retry). */
  userMessage: ChatMessage | null;
  /** The conversation's title (the first prompt names a new chat). */
  title: string;
}
