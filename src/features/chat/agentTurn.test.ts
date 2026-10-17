import { describe, expect, it, vi } from 'vitest';
import type { ToolCall } from '../../shared/types';
import { callTarget, createAgentTurn, isAgentEvent, statusText } from './agentTurn';

const read: ToolCall = { id: 'c1', name: 'read_file', input: { path: 'src/app.ts' } };
const run: ToolCall = { id: 'c2', name: 'run_command', input: { command: 'npm test' } };

describe('agent turn state', () => {
  it('labels calls by their main argument', () => {
    expect(callTarget(read)).toBe('src/app.ts');
    expect(callTarget(run)).toBe('npm test');
    expect(callTarget({ id: 'c3', name: 'list_dir', input: {} })).toBe('.');
    expect(statusText(read)).toBe('Reading src/app.ts');
    expect(statusText(run)).toBe('Running npm test');
    expect(statusText({ id: 'c4', name: 'other', input: {} })).toBe('other');
  });

  it('tracks live tools, reasoning, permission and notices', () => {
    const phase = vi.fn();
    const turn = createAgentTurn(phase);
    expect(isAgentEvent({ type: 'delta', text: 'x' })).toBe(false);
    turn.handle({ type: 'reasoning', text: 'Look ' });
    turn.handle({ type: 'reasoning', text: 'first.' });
    expect(turn.reasoning()).toBe('Look first.');
    turn.handle({ type: 'toolStart', call: read, status: 'Reading src/app.ts' });
    expect(turn.status()).toBe('Reading src/app.ts');
    expect(phase).toHaveBeenLastCalledWith('work');
    turn.handle({ type: 'toolEnd', callId: 'c1', ok: true, summary: '12 lines' });
    expect(turn.tools()).toEqual([{ call: read, ok: true, summary: '12 lines' }]);
    turn.handle({
      type: 'permission',
      request: {
        id: 7,
        callId: 'c2',
        tool: 'run_command',
        input: run.input,
        category: 'command',
        dangerous: null,
        preview: null,
      },
    });
    expect(turn.permission()?.id).toBe(7);
    expect(phase).toHaveBeenLastCalledWith('wait');
    turn.handle({ type: 'notice', kind: 'maxIterations', message: 'Stopped after 25 steps' });
    turn.stepStored();
    expect(turn.tools()).toEqual([]);
    expect(turn.reasoning()).toBe('');
    turn.reset(true);
    expect(turn.permission()).toBeNull();
    expect(turn.notices()).toHaveLength(1);
    turn.reset();
    expect(turn.notices()).toHaveLength(0);
  });
});
