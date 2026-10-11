// AI providers, API chats and conversations (phase 5). Mirrors src-tauri/src/{settings,
// providers,conversations}. API keys never cross IPC: the frontend only sees KeyStatus.

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

export interface ChatSettings {
  /** Streamed text is revealed smoothly; off = shown as it arrives. */
  smoothReveal: boolean;
}

/** All the frontend ever learns about a stored API key. */
export interface KeyStatus {
  hasKey: boolean;
  last4: string | null;
}

export interface ModelInfo {
  id: string;
  name: string | null;
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

export type MessageRole = 'user' | 'assistant';
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
}

export interface ConversationPatch {
  title?: string;
  providerId?: string | null;
  model?: string;
  systemPrompt?: string;
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

/** Events of one streamed reply, in order: deltas and usage, then exactly one stop or error. */
export type ChatEvent =
  | { type: 'delta'; text: string }
  | { type: 'usage'; usage: TokenUsage }
  | { type: 'stop'; reason: string; message: ChatMessage | null }
  | { type: 'error'; error: ChatError; message: ChatMessage | null };

export interface ChatSendRequest {
  workspace: string | null;
  conversationId: string;
  /** `null` = retry: resend the last user message (a failed reply after it is dropped). */
  text: string | null;
  attachments: Attachment[];
}

export interface ChatStarted {
  requestId: number;
  /** The stored user message (null on retry). */
  userMessage: ChatMessage | null;
}
