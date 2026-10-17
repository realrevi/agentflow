import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ChatMessage, CheckpointSummary } from '../../../shared/types';
import { wroteFiles } from './agentSession';
import { createCheckpoints, type CheckpointDeps } from './checkpoints';

// showToast schedules its dismissal on `window`.
vi.stubGlobal('window', globalThis);

function deps(answer: string) {
  let stored: CheckpointSummary[] = [
    { turn: 'u1', at: 1, files: ['a.md', 'new.txt'], revertedAt: null },
  ];
  const fake = {
    list: vi.fn(async () => stored),
    restore: vi.fn(async () => {
      stored = stored.map((c) => ({ ...c, revertedAt: 2 }));
      return { restored: ['a.md'], removed: ['new.txt'] };
    }),
    confirm: vi.fn(async () => answer),
    changed: vi.fn(),
  } satisfies CheckpointDeps;
  return fake;
}

describe('undo a turn', () => {
  let fake: ReturnType<typeof deps>;
  beforeEach(() => {
    fake = deps('undo');
  });

  it('confirms, restores, and then shows the turn as undone', async () => {
    const checkpoints = createCheckpoints('/w', 'c1', fake);
    await checkpoints.refresh();
    expect(checkpoints.byTurn('u1')?.revertedAt).toBeNull();
    expect(await checkpoints.undo('u1')).toBe(true);
    expect(fake.confirm).toHaveBeenCalledOnce();
    expect(fake.restore).toHaveBeenCalledWith('/w', 'c1', 'u1');
    expect(fake.changed).toHaveBeenCalledWith(['a.md', 'new.txt']);
    expect(checkpoints.byTurn('u1')?.revertedAt).toBe(2);
    expect(await checkpoints.undo('u1')).toBe(false);
    expect(fake.restore).toHaveBeenCalledOnce();
  });

  it('does nothing when the dialog is cancelled', async () => {
    const cancel = deps('cancel');
    const checkpoints = createCheckpoints('/w', 'c1', cancel);
    await checkpoints.refresh();
    expect(await checkpoints.undo('u1')).toBe(false);
    expect(cancel.restore).not.toHaveBeenCalled();
    expect(checkpoints.byTurn('u1')?.revertedAt).toBeNull();
  });

  it('notices a stored step that wrote files', () => {
    const base = { attachments: [], at: 1, status: 'complete' as const, stopReason: null };
    const step: ChatMessage = {
      ...base,
      id: 'a1',
      role: 'assistant',
      content: '',
      error: null,
      usage: null,
      parts: [
        { type: 'toolCall', id: 'w', name: 'write_file', input: {} },
        { type: 'toolCall', id: 'r', name: 'read_file', input: {} },
      ],
    };
    const results = (callId: string, outcome?: 'denied'): ChatMessage => ({
      ...base,
      id: 't1',
      role: 'tool',
      content: '',
      error: null,
      usage: null,
      parts: [{ type: 'toolResult', callId, content: '', isError: outcome !== undefined, outcome }],
    });
    expect(wroteFiles(results('w'), [step])).toBe(true);
    expect(wroteFiles(results('r'), [step])).toBe(false);
    expect(wroteFiles(results('w', 'denied'), [step])).toBe(false);
  });
});
