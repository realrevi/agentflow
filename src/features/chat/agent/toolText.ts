// One-line text for tool calls: the target, the state and the localized summary
// ("Read src/app.ts (lines 1–120)", "Ran npm test · exit 0 · 4.2 s"), plus durations.
import type { ToolCall, ToolResult } from '../../../shared/types';
import type { LiveTool } from './liveTurn';
import { locale, t, type I18nKey } from '../../i18n';

/** Read-only tools: consecutive calls group into one "Explored" row. */
export const READ_TOOLS: ReadonlySet<string> = new Set(['list_dir', 'read_file', 'glob', 'search']);

/** `pending`: no result and nothing running (waiting for approval, or interrupted). */
export type ToolState = 'running' | 'pending' | 'done' | 'error' | 'denied' | 'skipped';

const ACTIVE: Record<string, I18nKey> = {
  list_dir: 'agent.tool.list.active',
  read_file: 'agent.tool.read.active',
  glob: 'agent.tool.glob.active',
  search: 'agent.tool.search.active',
  write_file: 'agent.tool.write.active',
  edit_file: 'agent.tool.edit.active',
  run_command: 'agent.tool.run.active',
};

const MAX_TARGET = 80;
/** Durations under this are noise in a summary line. */
const SHOW_DURATION_MS = 1000;

/** The stored result, else the live one (a call that just ended or was denied). */
export function effectiveResult(
  stored: ToolResult | undefined,
  live: LiveTool | undefined,
): ToolResult | undefined {
  return stored ?? live?.result ?? undefined;
}

export function toolState(result: ToolResult | null | undefined, running: boolean): ToolState {
  if (!result) return running ? 'running' : 'pending';
  if (result.outcome === 'denied') return 'denied';
  if (result.outcome === 'skipped') return 'skipped';
  return result.isError ? 'error' : 'done';
}

function text(input: Record<string, unknown>, key: string): string | null {
  const value = input[key];
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

/** The main argument: the command, pattern, regex or path. */
export function toolTarget(call: ToolCall): string {
  const input = call.input;
  const key: Record<string, string> = { run_command: 'command', glob: 'pattern', search: 'regex' };
  const raw = text(input, key[call.name] ?? 'path') ?? '.';
  const line = raw.replace(/\s+/g, ' ');
  return line.length > MAX_TARGET ? `${line.slice(0, MAX_TARGET - 1)}…` : line;
}

/** "Reading src/app.ts", "Running npm test": while it runs, waits or did not run. */
export function activeText(call: ToolCall): string {
  const key = ACTIVE[call.name];
  return key ? t(key, { target: toolTarget(call) }) : call.name;
}

/** "4.2 s", "850 ms", "2 min" in the UI language. */
export function durationText(ms: number): string {
  const format = (value: number, unit: string, digits = 0) =>
    new Intl.NumberFormat(locale(), {
      style: 'unit',
      unit,
      unitDisplay: 'narrow',
      maximumFractionDigits: digits,
    }).format(value);
  if (ms < 1000) return format(Math.round(ms), 'millisecond');
  if (ms < 60_000) return format(ms / 1000, 'second', 1);
  return format(Math.round(ms / 60_000), 'minute');
}

/** What a finished call did. */
function doneText(call: ToolCall, result: ToolResult): string {
  const target = toolTarget(call);
  const meta = result.meta ?? {};
  switch (call.name) {
    case 'read_file': {
      const [first, last, total] = meta.lines ?? [0, 0, 0];
      if (!meta.lines || (first <= 1 && last >= total)) {
        return t('agent.tool.read.done', { target, count: total });
      }
      return t('agent.tool.read.range', { target, first, last });
    }
    case 'list_dir':
      return t('agent.tool.list.done', { target, count: meta.count ?? 0 });
    case 'glob':
      return t('agent.tool.glob.done', { target, count: meta.count ?? 0 });
    case 'search':
      return t('agent.tool.search.done', {
        target,
        count: meta.count ?? 0,
        files: meta.files ?? 0,
      });
    case 'write_file':
      return t(meta.created ? 'agent.tool.write.created' : 'agent.tool.write.done', {
        target,
        count: meta.count ?? 0,
      });
    case 'edit_file':
      return t('agent.tool.edit.done', { target, count: meta.count ?? 1 });
    case 'run_command':
      return meta.exitCode === undefined
        ? t('agent.tool.run.done', { target })
        : t('agent.tool.run.exit', { target, code: meta.exitCode });
    default:
      return call.name;
  }
}

/** The card's one-line summary. */
export function toolSummary(call: ToolCall, result: ToolResult | null | undefined): string {
  if (!result || result.outcome === 'denied' || result.outcome === 'skipped') {
    return activeText(call);
  }
  // A failed run still says what it tried; commands keep their exit code.
  const base =
    result.isError && result.meta?.exitCode === undefined
      ? activeText(call)
      : doneText(call, result);
  const ms = result.durationMs;
  return ms !== undefined && ms >= SHOW_DURATION_MS ? `${base} · ${durationText(ms)}` : base;
}
