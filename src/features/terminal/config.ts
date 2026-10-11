// What a terminal panel runs. Persisted in the layout (never its output), so it is
// parsed defensively when a saved layout is restored.
import type { PtyKind } from '../../shared/types';
import { t, type I18nKey } from '../i18n';

export interface TerminalConfig {
  kind: PtyKind;
  /** Custom terminals only: the command line to run. */
  command: string | null;
  args: string[];
  /** Folder to start in; `null` = home directory. */
  cwd: string | null;
}

export const AGENT_KINDS = ['claude', 'codex', 'gemini'] as const satisfies readonly PtyKind[];
export type AgentKind = (typeof AGENT_KINDS)[number];

const KINDS: readonly PtyKind[] = ['shell', ...AGENT_KINDS, 'custom'];
const TITLE_MAX = 32;

const LABELS: Record<PtyKind, I18nKey> = {
  shell: 'terminal.kind.shell',
  claude: 'terminal.kind.claude',
  codex: 'terminal.kind.codex',
  gemini: 'terminal.kind.gemini',
  custom: 'terminal.kind.custom',
};

export function isPtyKind(value: unknown): value is PtyKind {
  return typeof value === 'string' && (KINDS as readonly string[]).includes(value);
}

export function isAgentKind(kind: PtyKind): kind is AgentKind {
  return (AGENT_KINDS as readonly string[]).includes(kind);
}

export function kindLabel(kind: PtyKind): string {
  return t(LABELS[kind]);
}

/** Tab title: the kind's name, or the command itself for custom terminals. */
export function terminalTitle(config: TerminalConfig): string {
  if (config.kind !== 'custom' || !config.command) return kindLabel(config.kind);
  const line = [config.command, ...config.args].join(' ');
  return line.length > TITLE_MAX ? `${line.slice(0, TITLE_MAX - 1)}…` : line;
}

export function parseTerminalConfig(raw: unknown): TerminalConfig | null {
  if (typeof raw !== 'object' || raw === null) return null;
  const { kind, command, args, cwd } = raw as Record<string, unknown>;
  if (!isPtyKind(kind)) return null;
  const text = (value: unknown) => (typeof value === 'string' && value.trim() ? value : null);
  const parsedArgs = Array.isArray(args)
    ? args.filter((arg): arg is string => typeof arg === 'string')
    : [];
  const config = { kind, command: text(command), args: parsedArgs, cwd: text(cwd) };
  return kind === 'custom' && !config.command ? null : config;
}
