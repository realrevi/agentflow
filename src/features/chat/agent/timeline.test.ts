import { describe, expect, it } from 'vitest';
import type { ChatMessage, Part } from '../../../shared/types';
import { buildTimeline, reconcile } from './timeline';

let clock = 0;
function message(role: ChatMessage['role'], content: string, parts: Part[] = []): ChatMessage {
  clock += 1;
  return {
    id: `${role[0]}${clock}`,
    role,
    content,
    attachments: [],
    at: clock,
    status: 'complete',
    stopReason: null,
    error: null,
    usage: null,
    parts,
  };
}
const callPart = (id: string, name: string, input: Record<string, unknown> = {}): Part => ({
  type: 'toolCall',
  id,
  name,
  input,
});
const resultPart = (callId: string): Part => ({
  type: 'toolResult',
  callId,
  content: 'ok',
  isError: false,
});
const thought = (text: string, durationMs?: number): Part => ({
  type: 'reasoning',
  text,
  signature: null,
  redacted: null,
  details: null,
  durationMs,
});

describe('agent timeline', () => {
  it('groups consecutive reads, keeps writes and text apart', () => {
    const user = message('user', 'look around');
    const messages = [
      user,
      message('assistant', '', [thought('first', 1000), callPart('a', 'list_dir')]),
      message('tool', '', [resultPart('a')]),
      message('assistant', '', [thought('then', 500), callPart('b', 'read_file', { path: 'x' })]),
      message('tool', '', [resultPart('b')]),
      message('assistant', 'Now I edit.', [callPart('c', 'edit_file', { path: 'x' })]),
      message('tool', '', [resultPart('c')]),
      message('assistant', 'Done.'),
    ];
    const kinds = buildTimeline(messages, null, null).map((item) => item.kind);
    expect(kinds).toEqual(['user', 'reasoning', 'group', 'reply', 'tool', 'reply', 'turnEnd']);
    const items = buildTimeline(messages, null, null);
    const reasoning = items[1];
    expect(reasoning?.kind === 'reasoning' && reasoning.text).toBe('first\n\nthen');
    expect(reasoning?.kind === 'reasoning' && reasoning.durationMs).toBe(1500);
    const group = items[2];
    expect(group?.kind === 'group' && group.tools.map((t) => t.result?.callId)).toEqual(['a', 'b']);
    expect(items.at(-1)).toMatchObject({ kind: 'turnEnd', turn: user.id });
  });

  it('keeps a single read as its own card and shows no turn end without writes', () => {
    const messages = [
      message('user', 'read'),
      message('assistant', '', [callPart('r', 'read_file', { path: 'a' })]),
      message('tool', '', [resultPart('r')]),
      message('assistant', 'It says hi.'),
    ];
    expect(buildTimeline(messages, null, null).map((item) => item.kind)).toEqual([
      'user',
      'tool',
      'reply',
    ]);
  });

  it('shows the end of a turn only once it stopped streaming', () => {
    const user = message('user', 'write');
    const messages = [
      user,
      message('assistant', '', [callPart('w', 'write_file', { path: 'n.md' })]),
      message('tool', '', [resultPart('w')]),
    ];
    expect(buildTimeline(messages, null, user.id).some((i) => i.kind === 'turnEnd')).toBe(false);
    expect(buildTimeline(messages, null, null).at(-1)?.kind).toBe('turnEnd');
  });

  it('reuses unchanged items so the list does not re-render them', () => {
    const messages = [
      message('user', 'go'),
      message('assistant', '', [callPart('a', 'run_command', { command: 'ls' })]),
    ];
    const first = reconcile([], buildTimeline(messages, null, messages[0]?.id ?? null));
    const tool = first[1];
    const again = reconcile(first, buildTimeline(messages, null, messages[0]?.id ?? null));
    expect(again[1]).toBe(tool);
    const stored = [...messages, message('tool', '', [resultPart('a')])];
    const next = reconcile(again, buildTimeline(stored, null, null));
    expect(next[0]).toBe(first[0]);
    expect(next[1]).not.toBe(tool);
    expect(next[1]?.kind === 'tool' && next[1].result?.callId).toBe('a');
  });
});
