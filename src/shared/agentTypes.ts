// API agents (phase 7). Mirrors src-tauri/src/{agent,providers/types.rs,settings/agent.rs}.

/** A tool call the model made (`input` is `{ _raw }` when it sent invalid JSON). */
export interface ToolCall {
  id: string;
  name: string;
  input: Record<string, unknown>;
}

export interface ToolResult {
  callId: string;
  content: string;
  isError: boolean;
  /** Missing = `ran`. */
  outcome?: ToolOutcome;
  /** How long the run took (missing when it never ran). */
  durationMs?: number;
  meta?: ToolMeta;
}

/** `denied`: the user said no; `skipped`: never ran (turn stopped, not in this mode). */
export type ToolOutcome = 'ran' | 'denied' | 'skipped';

/** Numbers the chat builds its summary lines from. */
export interface ToolMeta {
  /** Entries, files, matches, lines written or replacements, per tool. */
  count?: number;
  /** Files with matches (search). */
  files?: number;
  /** [first, last, total] lines (read_file). */
  lines?: [number, number, number];
  /** write_file created the file. */
  created?: boolean;
  exitCode?: number;
}

/** Reasoning kept for the provider (Anthropic signature, OpenRouter details). */
export interface Reasoning {
  text: string;
  signature: string | null;
  redacted: string | null;
  details: unknown;
  /** How long the model thought. */
  durationMs?: number;
}

/** A piece of an agent message. */
export type Part =
  | { type: 'text'; text: string }
  | ({ type: 'reasoning' } & Reasoning)
  | ({ type: 'toolCall' } & ToolCall)
  | ({ type: 'toolResult' } & ToolResult);

/** Ask: reads run, writes and commands ask. Auto-edit: commands ask. Read-only: reads only. */
export type PermissionMode = 'ask' | 'autoEdit' | 'readOnly';

export interface AgentState {
  /** `null` = the default from settings. */
  permissionMode: PermissionMode | null;
  /** Tools allowed "for this conversation" (dangerous commands still ask). */
  allowedTools: string[];
  /** The provider rejected tools for this model: the chat continues without them. */
  toolsDisabled: boolean;
  /** The environment part of the default system prompt (built once). */
  context: string | null;
  /** The installed AgentFlow agent this chat was opened with. */
  profile?: ProfileSnapshot;
}

/** What a chat keeps of an AgentFlow agent profile. */
export interface ProfileSnapshot {
  id: string;
  name: string;
  instructions: string;
  /** Only these tools (within what the permission mode allows). */
  tools: string[] | null;
}

export type ToolCategory = 'read' | 'write' | 'command';

export interface Preview {
  path: string;
  /** `null` = the file does not exist yet. */
  before: string | null;
  after: string;
}

export interface PermissionRequest {
  /** Answer with `agentPermissionAnswer(id, ...)`. */
  id: number;
  callId: string;
  tool: string;
  input: Record<string, unknown>;
  category: ToolCategory;
  /** Why the command is dangerous (it asks even when the tool is allowed). */
  dangerous: string | null;
  preview: Preview | null;
}

export type PermissionAnswer =
  | { decision: 'allowOnce' }
  | { decision: 'allowConversation' }
  | { decision: 'deny'; reason: string | null };

/** What the character does: a model call, a tool run, waiting for approval. */
export type AgentPhase = 'think' | 'work' | 'wait';

export interface CheckpointSummary {
  /** The id of the user message that started the turn. */
  turn: string;
  at: number;
  files: string[];
  /** When the turn was undone. */
  revertedAt: number | null;
}

export interface RestoreReport {
  restored: string[];
  removed: string[];
}

export interface ModelTools {
  model: string;
  tools: boolean;
}

export interface AgentSettings {
  maxIterations: number;
  /** Bytes of tool output sent to the model. */
  toolResultCap: number;
  commandTimeoutSecs: number;
  defaultMode: PermissionMode;
  defaultPrompt: boolean;
  /** Show the model's reasoning in chats. */
  showReasoning: boolean;
  /** Per-model "supports tools" overrides. */
  toolModels: ModelTools[];
}
