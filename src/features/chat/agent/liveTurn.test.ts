import { describe, expect, it, vi } from 'vitest';
import { ipc } from '../../../shared/ipc';
import type { PermissionRequest, ToolCall } from '../../../shared/types';
import { createLiveTurn } from './liveTurn';
import { reasoningOpen, reasoningTitle } from './reasoning';
import { effectiveResult, toolState } from './toolText';

const write: ToolCall = { id: 'w1', name: 'write_file', input: { path: 'a.md', content: 'x' } };
const request: PermissionRequest = {
  id: 3,
  callId: 'w1',
  tool: 'write_file',
  input: write.input,
  category: 'write',
  dangerous: null,
  preview: { path: 'a.md', before: null, after: 'x' },
};
const hooks = () => ({ phase: vi.fn(), asked: vi.fn() });

describe('live agent turn', () => {
  it('shows a denied call as ✗ at once, before its tool message is stored', () => {
    const turn = createLiveTurn('c1', hooks());
    turn.handle({ type: 'permission', request });
    turn.handle({
      type: 'toolEnd',
      result: {
        callId: 'w1',
        content: 'The user denied this tool call.',
        isError: true,
        outcome: 'denied',
      },
    });
    const live = turn.tool('w1');
    expect(toolState(effectiveResult(undefined, live), live?.startedAt != null)).toBe('denied');
    expect(turn.permission()).toBeNull();
    turn.stepStored('tool');
    expect(turn.tool('w1')).toBeUndefined();
  });

  it('tracks running calls and the bubble text', () => {
    const phase = vi.fn();
    const turn = createLiveTurn('c1', { phase, asked: vi.fn() });
    turn.handle({ type: 'status', phase: 'work' });
    turn.handle({ type: 'toolStart', call: write });
    expect(toolState(effectiveResult(undefined, turn.tool('w1')), true)).toBe('running');
    expect(turn.status()).toBe('Writing a.md');
    expect(phase).toHaveBeenCalledWith('work');
  });

  it('collapses live reasoning when the answer starts and shows how long it took', () => {
    let now = 1000;
    const turn = createLiveTurn('c1', hooks(), () => now);
    turn.handle({ type: 'reasoning', text: 'Let me ' });
    turn.handle({ type: 'reasoning', text: 'look.' });
    const thinking = { live: true, answering: false, durationMs: undefined };
    expect(turn.reasoning()?.text).toBe('Let me look.');
    expect(reasoningOpen(thinking, null)).toBe(true);
    expect(reasoningTitle(thinking)).toBe('Thinking…');
    now = 4200;
    turn.answerStarted();
    const live = turn.reasoning();
    const done = {
      live: true,
      answering: live?.endedAt !== null,
      durationMs: (live?.endedAt ?? 0) - (live?.startedAt ?? 0),
    };
    expect(reasoningOpen(done, null)).toBe(false);
    expect(reasoningOpen(done, true)).toBe(true);
    expect(reasoningTitle(done)).toBe('Thought for 3.2s');
    turn.stepStored('assistant');
    expect(turn.reasoning()).toBeNull();
  });

  it('answers only from a trusted event on the pending request, with its conversation', async () => {
    const send = vi.spyOn(ipc, 'agentPermissionAnswer').mockResolvedValue(true);
    const asked = vi.fn();
    const turn = createLiveTurn('c1', { phase: vi.fn(), asked });
    turn.handle({ type: 'permission', request });
    expect(asked).toHaveBeenCalledWith(request);
    const trusted = { isTrusted: true } as Event;
    await turn.answer(3, { decision: 'allowOnce' }, new Event('click'));
    await turn.answer(2, { decision: 'allowOnce' }, trusted);
    expect(send).not.toHaveBeenCalled();
    await turn.answer(3, { decision: 'deny', reason: 'no' }, trusted);
    expect(send).toHaveBeenCalledWith(3, 'c1', { decision: 'deny', reason: 'no' });
    await turn.answer(3, { decision: 'allowOnce' }, trusted);
    expect(send).toHaveBeenCalledTimes(1);
    send.mockRestore();
  });

  it('keeps the step-limit notice after the turn and offers a continue prompt', () => {
    const turn = createLiveTurn('c1', hooks());
    turn.handle({ type: 'notice', kind: 'maxIterations', message: 'Stopped after 25 steps' });
    turn.reset(true);
    expect(turn.notices()).toEqual([{ kind: 'maxIterations', detail: 'Stopped after 25 steps' }]);
    expect(turn.continuePrompt()).toBe('Continue where you stopped.');
    turn.reset();
    expect(turn.notices()).toEqual([]);
  });
});
