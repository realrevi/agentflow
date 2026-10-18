import { describe, expect, it } from 'vitest';
import type { HandoffMode } from '../../shared/types';
import type { TurnEnd } from '../agentui/turns';
import { step, type Action, type Input, type Step } from './engine';
import type { Workstream } from './model';

const fence = '```';
const limits = { maxRounds: 2, reportCap: 30 * 1024 };
let ids = 0;
const newId = () => `h${++ids}`;

function ws(mode: HandoffMode, patch: Partial<Workstream> = {}): Workstream {
  return {
    id: 'ws-1',
    name: 'Feature',
    color: '#F5C542',
    mode,
    manager: 'chat-1',
    workers: ['terminal-2', 'terminal-3'],
    names: { 'chat-1': 'Owl', 'terminal-2': 'Fox', 'terminal-3': 'Panda' },
    status: 'idle',
    reason: null,
    detail: null,
    warning: null,
    round: 0,
    setupSent: true,
    pending: null,
    awaiting: [],
    ...patch,
  };
}

function turn(panel: string, report: string, extra: Partial<TurnEnd> = {}): Input {
  return {
    type: 'turn',
    end: {
      panel,
      turn: 1,
      report,
      source: 'stop',
      interrupted: false,
      approximate: false,
      at: 0,
      ...extra,
    },
  };
}

const send = (to: string, body = 'Do the task.') =>
  `Plan.\n${fence}agentflow-send to="${to}"\n${body}\n${fence}`;

function run(start: Workstream, inputs: Input[]): Step {
  let current: Step = { ws: start, actions: [] };
  const actions: Action[] = [];
  for (const input of inputs) {
    current = step(current.ws, input, limits, newId);
    actions.push(...current.actions);
  }
  return { ws: current.ws, actions };
}

const deliveries = (actions: Action[]) =>
  actions.flatMap((a) => (a.type === 'deliver' ? [[a.handoff.kind, a.handoff.to]] : []));

describe('workstream modes', () => {
  it('semi-auto holds each handoff as a preview; one click sends it', () => {
    const first = run(ws('semiAuto'), [turn('chat-1', send('Fox'))]);
    expect(first.ws.status).toBe('waiting');
    expect(first.ws.pending).toMatchObject({
      kind: 'prompt',
      to: 'terminal-2',
      text: 'Do the task.',
    });
    expect(deliveries(first.actions)).toEqual([]);
    expect(first.actions).toContainEqual({ type: 'attention', panel: 'chat-1', reason: 'turn' });

    const sent = run(first.ws, [{ type: 'send', text: 'Do the task, edited.' }]);
    expect(sent.actions).toEqual([
      { type: 'deliver', handoff: expect.objectContaining({ text: 'Do the task, edited.' }) },
    ]);
    expect(sent.ws.awaiting).toEqual(['terminal-2']);

    // The worker's report comes back as a preview for the manager.
    const report = run(sent.ws, [turn('terminal-2', 'Added the test.')]);
    expect(report.ws.pending).toMatchObject({
      kind: 'report',
      to: 'chat-1',
      text: 'Report from Fox (turn 1):\n\nAdded the test.',
    });
  });

  it('manual mode waits for the button too', () => {
    const result = run(ws('manual'), [turn('chat-1', send('Panda'))]);
    expect(result.ws.pending?.to).toBe('terminal-3');
    expect(deliveries(result.actions)).toEqual([]);
  });

  it('auto sends at once, both ways', () => {
    const result = run(ws('auto', { status: 'running' }), [
      turn('chat-1', send('Fox')),
      turn('terminal-2', 'Done with it.'),
    ]);
    expect(deliveries(result.actions)).toEqual([
      ['prompt', 'terminal-2'],
      ['report', 'chat-1'],
    ]);
    expect(result.ws.round).toBe(1);
  });

  it('a worker turn nobody asked for is not forwarded', () => {
    const result = run(ws('auto', { status: 'running' }), [turn('terminal-3', 'chatting')]);
    expect(result.actions).toEqual([]);
  });
});

describe('auto stop conditions', () => {
  const running = () => ws('auto', { status: 'running', awaiting: ['terminal-2'] });

  it('stops on an ask or done from the manager', () => {
    const asked = run(running(), [turn('chat-1', `${fence}agentflow-ask\nWhich DB?\n${fence}`)]);
    expect([asked.ws.status, asked.ws.reason, asked.ws.detail]).toEqual([
      'waiting',
      'ask',
      'Which DB?',
    ]);
    expect(asked.actions).toEqual([{ type: 'attention', panel: 'chat-1', reason: 'question' }]);
    const done = run(running(), [turn('chat-1', `${fence}agentflow-done\nShipped.\n${fence}`)]);
    expect([done.ws.status, done.ws.reason]).toEqual(['done', 'done']);
  });

  it('stops on any pending permission (never answered for the user)', () => {
    const result = run(running(), [{ type: 'permission', panel: 'terminal-2' }]);
    expect([result.ws.status, result.ws.reason]).toEqual(['paused', 'permission']);
    expect(result.actions).toEqual([]);
    // Answered by the user; the turn ends and its report waits for a click.
    const after = run(result.ws, [turn('terminal-2', 'Done.')]);
    expect([after.ws.status, after.ws.reason, after.ws.pending?.kind]).toEqual([
      'paused',
      'review',
      'report',
    ]);
    expect(after.actions.some((a) => a.type === 'deliver')).toBe(false);
  });

  it('stops on an error or a malformed reply, and on no block at all', () => {
    expect(run(running(), [{ type: 'error', panel: 'chat-1', message: 'boom' }]).ws.reason).toBe(
      'error',
    );
    const malformed = run(running(), [turn('chat-1', `${fence}agentflow-send\nx\n${fence}`)]);
    expect([malformed.ws.status, malformed.ws.reason]).toEqual(['paused', 'malformed']);
    expect(run(running(), [turn('chat-1', 'no block')]).ws.reason).toBe('noBlock');
  });

  it('pauses on an unknown worker with an error', () => {
    const result = run(running(), [turn('chat-1', send('Wolf'))]);
    expect([result.ws.reason, result.ws.detail]).toEqual(['unknownWorker', 'Wolf']);
    expect(result.actions).toEqual([{ type: 'attention', panel: 'chat-1', reason: 'error' }]);
  });

  it('stops at max rounds and keeps the handoff for the user', () => {
    const result = run(ws('auto', { status: 'running' }), [
      turn('chat-1', send('Fox', 'one')),
      turn('terminal-2', 'r1'),
      turn('chat-1', send('Fox', 'two')),
      turn('terminal-2', 'r2'),
      turn('chat-1', send('Fox', 'three')),
    ]);
    expect(deliveries(result.actions).filter(([kind]) => kind === 'prompt')).toHaveLength(2);
    expect([result.ws.status, result.ws.reason, result.ws.pending?.text]).toEqual([
      'paused',
      'maxRounds',
      'three',
    ]);
    // Sending it by hand starts a new run.
    const resumed = run(result.ws, [{ type: 'send' }]);
    expect(resumed.ws.round).toBe(1);
  });

  it('stops on a budget hit, the user typing into a member, and Stop', () => {
    expect(run(running(), [{ type: 'budget' }]).ws.reason).toBe('budget');
    expect(run(running(), [{ type: 'typed', panel: 'terminal-2' }]).ws.reason).toBe('userTyped');
    expect(run(running(), [{ type: 'typed', panel: 'terminal-9' }]).ws.status).toBe('running');
    expect(run(running(), [{ type: 'stop' }]).ws.reason).toBe('stopped');
  });

  it('a stopped workstream holds handoffs until resumed', () => {
    const stopped = run(running(), [{ type: 'stop' }, turn('terminal-2', 'late report')]);
    expect(stopped.ws.status).toBe('paused');
    expect(deliveries(stopped.actions)).toEqual([]);
    expect(stopped.ws.pending?.kind).toBe('report');
    const resumed = run(stopped.ws, [{ type: 'resume' }]);
    expect(deliveries(resumed.actions)).toEqual([['report', 'chat-1']]);
  });

  it('a failed delivery pauses and keeps the handoff to retry', () => {
    const result = run(ws('auto', { status: 'running' }), [turn('chat-1', send('Fox'))]);
    const handoff = result.actions.find((a) => a.type === 'deliver');
    if (handoff?.type !== 'deliver') throw new Error('no delivery');
    const failed = run(result.ws, [{ type: 'failed', handoff: handoff.handoff }]);
    expect([failed.ws.reason, failed.ws.pending?.id, failed.ws.round]).toEqual([
      'notRunning',
      handoff.handoff.id,
      0,
    ]);
  });
});

describe('CLI manager setup', () => {
  it('is offered once, like a handoff', () => {
    const fresh = ws('semiAuto', { setupSent: false });
    const result = run(fresh, [{ type: 'start', setup: 'instructions' }]);
    expect(result.ws.pending).toMatchObject({ kind: 'setup', to: 'chat-1' });
    const sent = run(result.ws, [{ type: 'send' }]);
    const handoff = sent.actions[0];
    if (handoff?.type !== 'deliver') throw new Error('no delivery');
    expect(run(sent.ws, [{ type: 'delivered', handoff: handoff.handoff }]).ws.setupSent).toBe(true);
    expect(run(ws('auto'), [{ type: 'start', setup: null }]).actions).toEqual([]);
  });

  it('is offered again when the CLI manager restarted (a new session)', () => {
    const restored = ws('auto', { setupSent: true, status: 'paused', reason: 'restored' });
    expect(run(restored, [{ type: 'start', setup: 'instructions' }]).ws.pending).toBeNull();
    const again = run(restored, [{ type: 'start', setup: 'instructions', fresh: true }]);
    expect(again.ws.pending?.kind).toBe('setup');
    expect([again.ws.status, again.ws.reason]).toEqual(['paused', 'restored']);
    expect(again.actions.some((a) => a.type === 'deliver')).toBe(false);
  });
});

describe('auto format reminder', () => {
  const remind = { ...limits, autoRemind: true };
  const block = `Plan.
${fence}agentflow-send to="Fox"
Fix it.
${fence}`;

  it('sends one reminder, then pauses on a second invalid reply', () => {
    const start = ws('auto', { status: 'running' });
    const first = step(start, turn('chat-1', 'no block'), remind, newId);
    expect(first.actions).toEqual([{ type: 'remind' }]);
    expect(first.ws.status).toBe('running');
    expect(first.ws.reminded).toBe(true);
    const second = step(first.ws, turn('chat-1', 'still none'), remind, newId);
    expect(second.ws.reason).toBe('noBlock');
    expect(second.actions.some((a) => a.type === 'remind')).toBe(false);
  });

  it('a valid reply after the reminder goes on and resets it', () => {
    const first = step(ws('auto', { status: 'running' }), turn('chat-1', 'oops'), remind, newId);
    const next = step(first.ws, turn('chat-1', block), remind, newId);
    expect(next.ws.reminded).toBe(false);
    expect(next.actions.some((a) => a.type === 'deliver')).toBe(true);
  });

  it('reminds on malformed blocks too, and never when turned off', () => {
    const bad = `${fence}agentflow-send
no target
${fence}`;
    const on = step(ws('semiAuto', { status: 'running' }), turn('chat-1', bad), remind, newId);
    expect(on.actions).toEqual([{ type: 'remind' }]);
    const off = step(ws('auto', { status: 'running' }), turn('chat-1', 'x'), limits, newId);
    expect(off.ws.reason).toBe('noBlock');
  });
});
