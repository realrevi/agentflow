// The conversation as render items: prompts, reasoning, reply text, one item per tool call
// (with its stored result), runs of read-only calls grouped into one "Explored" row, and
// the end of each turn that changed files. `reconcile` keeps unchanged items identical so
// <For> does not re-render them.
import type { ChatMessage, Compaction, ToolCall, ToolResult } from '../../../shared/types';
import { READ_TOOLS } from './toolText';

export interface ToolEntry {
  call: ToolCall;
  /** The stored result (`undefined` until the step's tool message is stored). */
  result: ToolResult | undefined;
}

export type TimelineItem =
  | { kind: 'user'; key: string; message: ChatMessage }
  | { kind: 'reasoning'; key: string; text: string; durationMs: number | undefined }
  | { kind: 'reply'; key: string; message: ChatMessage }
  | ({ kind: 'tool'; key: string } & ToolEntry)
  | { kind: 'group'; key: string; tools: readonly ToolEntry[] }
  | { kind: 'turnEnd'; key: string; turn: string }
  | { kind: 'compaction'; key: string; compaction: Compaction };

/** Fewer read-only calls than this in a row stay separate cards. */
export const MIN_GROUP = 2;
const WRITE_TOOLS: ReadonlySet<string> = new Set(['write_file', 'edit_file']);

/** Tool results by call id, from the `tool` messages. */
export function resultsOf(messages: readonly ChatMessage[]): Map<string, ToolResult> {
  const results = new Map<string, ToolResult>();
  for (const message of messages) {
    if (message.role !== 'tool') continue;
    for (const part of message.parts ?? []) {
      if (part.type === 'toolResult') results.set(part.callId, part);
    }
  }
  return results;
}

function stepItems(message: ChatMessage, results: Map<string, ToolResult>): TimelineItem[] {
  const items: TimelineItem[] = [];
  const parts = message.parts ?? [];
  const thinking = parts.flatMap((part) => (part.type === 'reasoning' ? [part] : []));
  const thought = thinking
    .map((part) => part.text)
    .join('\n')
    .trim();
  if (thought) {
    const durationMs = thinking.find((part) => part.durationMs !== undefined)?.durationMs;
    items.push({ kind: 'reasoning', key: `${message.id}:r`, text: thought, durationMs });
  }
  const calls = parts.flatMap((part) => (part.type === 'toolCall' ? [part] : []));
  const hasReply = message.content.trim() !== '' || message.status === 'error';
  if (hasReply || (calls.length === 0 && !thought)) {
    items.push({ kind: 'reply', key: message.id, message });
  }
  // The stored part itself: the same object on every rebuild.
  for (const call of calls) {
    items.push({ kind: 'tool', key: call.id, call, result: results.get(call.id) });
  }
  return items;
}

/** Reasoning items of a grouped run, as one (texts joined, durations added). */
function merged(items: TimelineItem[]): TimelineItem[] {
  const thoughts = items.flatMap((item) => (item.kind === 'reasoning' ? [item] : []));
  const first = thoughts[0];
  if (!first) return [];
  const known = thoughts.filter((item) => item.durationMs !== undefined);
  return [
    {
      kind: 'reasoning',
      key: first.key,
      text: thoughts.map((item) => item.text).join('\n\n'),
      durationMs: known.length
        ? known.reduce((sum, item) => sum + (item.durationMs ?? 0), 0)
        : undefined,
    },
  ];
}

/** Runs of read-only calls; reasoning between them merges into one item before the group. */
function grouped(items: TimelineItem[]): TimelineItem[] {
  const out: TimelineItem[] = [];
  let run: TimelineItem[] = [];
  const flush = () => {
    const tools = run.flatMap((item) => (item.kind === 'tool' ? [item] : []));
    const last = tools[tools.length - 1];
    if (tools.length >= MIN_GROUP && last) {
      // Reasoning after the last call belongs to what follows.
      const end = run.indexOf(last) + 1;
      out.push(...merged(run.slice(0, end)), {
        kind: 'group',
        key: `g:${tools[0]?.key ?? ''}`,
        tools: tools.map(({ call, result }) => ({ call, result })),
      });
      out.push(...run.slice(end));
    } else {
      out.push(...run);
    }
    run = [];
  };
  for (const item of items) {
    const read = item.kind === 'tool' && READ_TOOLS.has(item.call.name);
    // Reasoning joins tentatively: it stays in place unless reads around it group.
    if (read || item.kind === 'reasoning') {
      run.push(item);
    } else {
      flush();
      out.push(item);
    }
  }
  flush();
  return out;
}

/**
 * `streamingTurn`: the user message id of the turn still running (its end is not shown yet).
 */
export function buildTimeline(
  messages: readonly ChatMessage[],
  compaction: Compaction | null,
  streamingTurn: string | null,
): TimelineItem[] {
  const results = resultsOf(messages);
  const items: TimelineItem[] = [];
  let turn: string | null = null;
  let wrote = false;
  const endTurn = () => {
    if (turn && wrote && turn !== streamingTurn) {
      items.push({ kind: 'turnEnd', key: `${turn}:end`, turn });
    }
    wrote = false;
  };
  for (const message of messages) {
    if (message.role === 'user') {
      endTurn();
      turn = message.id;
      items.push({ kind: 'user', key: message.id, message });
    } else if (message.role === 'assistant') {
      const step = stepItems(message, results);
      wrote ||= step.some((item) => item.kind === 'tool' && WRITE_TOOLS.has(item.call.name));
      items.push(...step);
    }
    if (compaction?.through === message.id) {
      items.push({ kind: 'compaction', key: `${message.id}:c`, compaction });
    }
  }
  endTurn();
  return grouped(items);
}

function same(a: TimelineItem, b: TimelineItem): boolean {
  const left = a as Record<string, unknown>;
  const right = b as Record<string, unknown>;
  const keys = Object.keys(left);
  if (keys.length !== Object.keys(right).length) return false;
  return keys.every((key) => {
    const x = left[key];
    const y = right[key];
    if (key !== 'tools') return x === y;
    const xs = x as readonly ToolEntry[];
    const ys = y as readonly ToolEntry[];
    return (
      xs.length === ys.length &&
      xs.every((entry, i) => entry.call === ys[i]?.call && entry.result === ys[i]?.result)
    );
  });
}

/** Reuses the previous object for every item that did not change. */
export function reconcile(previous: readonly TimelineItem[], next: TimelineItem[]): TimelineItem[] {
  const byKey = new Map(previous.map((item) => [item.key, item]));
  return next.map((item) => {
    const before = byKey.get(item.key);
    return before && same(before, item) ? before : item;
  });
}
