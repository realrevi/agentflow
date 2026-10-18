// Agent installer (phase 8). Mirrors src-tauri/src/agents/ (serde camelCase); the template
// format is documented in docs/agent-template.md and schemas/agent-template.schema.json.
import type { PermissionMode } from './agentTypes';

export type QuestionKind =
  'text' | 'multiline' | 'choice' | 'multiChoice' | 'color' | 'font' | 'boolean';

/** A string, a list (multiChoice) or a bool (boolean). */
export type AnswerValue = string | string[] | boolean;
export type Answers = Record<string, AnswerValue>;

export interface Question {
  id: string;
  label: string;
  help?: string;
  type: QuestionKind;
  default?: AnswerValue;
  options?: string[];
  /** A workspace detector key (see `Detection.values`). */
  detect?: string;
  required?: boolean;
  group?: string;
}

export interface TargetOptions {
  claudeCode?: { tools?: string[]; model?: string };
  codex?: { model?: string; reasoningEffort?: string; sandboxMode?: string };
  gemini?: Record<string, never>;
  cursor?: { globs?: string[]; alwaysApply?: boolean };
  agentflow?: {
    permissionMode?: PermissionMode;
    tools?: string[];
    provider?: string;
    model?: string;
  };
}

export interface AgentTemplate {
  schema: 1;
  id: string;
  name: string;
  description: string;
  version: string;
  author?: string;
  license?: string;
  tags?: string[];
  character?: string;
  language?: string;
  instructions: string;
  questions?: Question[];
  targets?: TargetOptions;
  /** Default `agent`; skills carry `files`, library templates their `source`. */
  kind?: TemplateKind;
  files?: TemplateFile[];
  source?: TemplateSource;
}

export type TemplateKind = 'agent' | 'skill' | 'rule' | 'command';

export interface TemplateFile {
  /** Relative to the skill folder. */
  path: string;
  /** Text, or base64 when `binary`. */
  content: string;
  binary?: boolean;
}

/** Where a library template came from (attribution and update checks). */
export interface TemplateSource {
  repo: string;
  path: string;
  commit: string;
  sha: string;
  license: string;
  verified?: boolean;
  format?: string;
}

export type InstallTarget = 'claudeCode' | 'codex' | 'gemini' | 'cursor' | 'agentflow';
export type InstallScope = 'project' | 'user';

export interface InstallRequest {
  template: AgentTemplate;
  answers: Answers;
  targets: InstallTarget[];
  scope: InstallScope;
}

export type PlanStatus = 'create' | 'update' | 'unchanged' | 'conflict' | 'remove';
export type ConflictKind = 'foreign' | 'edited';
export type FileKind = 'file' | 'section';

export interface PlannedFile {
  target: InstallTarget;
  scope: InstallScope;
  path: string;
  /** Workspace-relative, or `~/...` for user scope. */
  display: string;
  kind: FileKind;
  status: PlanStatus;
  conflict: ConflictKind | null;
  before: string | null;
  after: string;
  remove: boolean;
  created: boolean;
}

export interface InstallPlan {
  files: PlannedFile[];
  userScope: boolean;
}

export interface Confirmations {
  /** `display` paths of conflicts the user agreed to overwrite or remove. */
  overwrite: string[];
  userScope: boolean;
}

export interface InstalledFile {
  target: InstallTarget;
  scope: InstallScope;
  path: string;
  kind: FileKind;
  hash: string;
  created: boolean;
}

export interface InstalledAgent {
  id: string;
  template: AgentTemplate;
  answers: Answers;
  targets: InstallTarget[];
  scope: InstallScope;
  installedAt: number;
  updatedAt: number;
  files: InstalledFile[];
}

export interface InstallResult {
  plan: InstallPlan;
  installed: InstalledAgent;
}

export interface Detection {
  /** Detector key -> suggested answer. */
  values: Record<string, string>;
  existing: { target: InstallTarget; path: string }[];
}

/** An installed AgentFlow agent (`.agentflow/agents/<id>.md`). */
export interface AgentProfile {
  id: string;
  name: string;
  description: string;
  provider: string | null;
  model: string | null;
  permissionMode: PermissionMode | null;
  tools: string[] | null;
  character: string | null;
  instructions: string;
}

export interface GenerateRequest {
  workspace: string | null;
  providerId: string;
  model: string;
  description: string;
  language: string;
  /** "Customize with AI": adapt this template (its text stays as it is). */
  base?: AgentTemplate;
}

export interface GenerateResult {
  template: AgentTemplate;
  attempts: number;
}
