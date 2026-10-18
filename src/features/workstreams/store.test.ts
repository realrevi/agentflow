import { beforeEach, describe, expect, it } from 'vitest';
import type { HandoffEntry } from '../../shared/types';
import { step } from './engine';
import { canRoute, parseWorkstream, type Workstream } from './model';
import {
  createWorkstream,
  linkWorker,
  restoreWorkstreams,
  snapshotWorkstreams,
  unlinkPanel,
  workstreamOf,
  workstreams,
} from './store';
import { filterEntries } from './timeline';

const base = {
  name: 'Feature',
  color: '#5EC8E5',
  mode: 'semiAuto' as const,
  names: { 'chat-1': 'Owl', 'terminal-2': 'Fox' },
};

beforeEach(() => restoreWorkstreams([], () => true));

describe('workstream store', () => {
  it('a worker belongs to one workstream at most', () => {
    const a = createWorkstream({ ...base, manager: 'chat-1', workers: ['terminal-2'] });
    const b = createWorkstream({
      ...base,
      manager: 'chat-5',
      workers: ['terminal-2', 'terminal-3'],
    });
    expect(workstreamOf('terminal-2')?.id).toBe(a);
    expect(workstreams().find((ws) => ws.id === b)?.workers).toEqual(['terminal-3']);
    expect(linkWorker(b, 'terminal-2', 'Fox')).toBe(false);
    expect(linkWorker(b, 'terminal-4', 'Fox')).toBe(true);
  });

  it('unlinking a worker keeps the workstream; unlinking the manager ends it', () => {
    const id = createWorkstream({ ...base, manager: 'chat-1', workers: ['terminal-2'] });
    unlinkPanel('terminal-2');
    expect(workstreams().find((ws) => ws.id === id)?.workers).toEqual([]);
    unlinkPanel('chat-1');
    expect(workstreams()).toHaveLength(0);
  });

  it('survives a restart through the saved layout (restored paused, closed panels dropped)', () => {
    createWorkstream({ ...base, manager: 'chat-1', workers: ['terminal-2', 'terminal-9'] });
    const saved = JSON.parse(JSON.stringify(snapshotWorkstreams())) as unknown[];
    restoreWorkstreams([], () => true);
    expect(workstreams()).toHaveLength(0);
    restoreWorkstreams(saved, (panel) => panel !== 'terminal-9');
    const [ws] = workstreams();
    expect(ws).toMatchObject({
      name: 'Feature',
      manager: 'chat-1',
      workers: ['terminal-2'],
      names: base.names,
      mode: 'semiAuto',
      status: 'paused',
      reason: 'restored',
    });
    // A saved workstream whose manager is gone is dropped.
    restoreWorkstreams(saved, (panel) => panel !== 'chat-1');
    expect(workstreams()).toHaveLength(0);
  });

  it('ignores junk in the saved layout', () => {
    expect(parseWorkstream({ id: '../x', name: 'a', manager: 'chat-1' })).toBeNull();
    expect(parseWorkstream({ id: 'ws-1', name: '', manager: 'chat-1' })).toBeNull();
    const repaired = parseWorkstream({
      id: 'ws-1',
      name: 'A',
      manager: 'chat-1',
      workers: ['chat-1', 'terminal-2', 'terminal-2', 7],
      color: 'red',
      mode: 'turbo',
    });
    expect(repaired).toMatchObject({ workers: ['terminal-2'], mode: 'semiAuto', color: '#F5C542' });
  });
});

describe('routing and restore rules', () => {
  const ws = (patch: Partial<Workstream> = {}): Workstream => ({
    id: 'ws-1',
    name: 'Feature',
    color: '#F5C542',
    mode: 'semiAuto',
    manager: 'chat-1',
    workers: ['terminal-2'],
    names: base.names,
    status: 'paused',
    reason: 'restored',
    detail: null,
    warning: null,
    round: 0,
    setupSent: true,
    pending: null,
    awaiting: [],
    ...patch,
  });

  it('routes only between members of the same workstream', () => {
    expect(canRoute(ws(), 'chat-1', 'terminal-2')).toBe(true);
    expect(canRoute(ws(), 'terminal-2', 'chat-1')).toBe(true);
    expect(canRoute(ws(), 'chat-1', 'terminal-7')).toBe(false);
    expect(canRoute(ws(), 'chat-8', 'terminal-2')).toBe(false);
  });

  it('talking to a restored workstream wakes it (auto stays stopped until resumed)', () => {
    const end = {
      panel: 'chat-1',
      turn: 1,
      report: '```agentflow-send to="Fox"\nGo.\n```',
      source: 'stop' as const,
      interrupted: false,
      approximate: false,
      at: 0,
    };
    const limits = { maxRounds: 10, reportCap: 1024 };
    const semi = step(ws(), { type: 'turn', end }, limits, () => 'h1');
    expect([semi.ws.status, semi.ws.reason, semi.ws.pending?.to]).toEqual([
      'waiting',
      'review',
      'terminal-2',
    ]);
    const auto = step(ws({ mode: 'auto' }), { type: 'turn', end }, limits, () => 'h2');
    expect([auto.ws.status, auto.ws.reason]).toEqual(['paused', 'stopped']);
    expect(auto.actions.some((a) => a.type === 'deliver')).toBe(false);
  });

  it('the timeline filters by panel or by a manager-worker pair', () => {
    const entry = (id: string, from: string, to: string): HandoffEntry => ({
      id,
      workstream: 'ws-1',
      at: 0,
      from,
      to,
      fromName: from,
      toName: to,
      kind: 'prompt',
      mode: 'auto',
      size: 1,
      tokens: null,
      text: id,
      truncated: false,
    });
    const all = [
      entry('a', 'chat-1', 'terminal-2'),
      entry('b', 'terminal-2', 'chat-1'),
      entry('c', 'chat-1', 'terminal-3'),
    ];
    expect(filterEntries(all, null, null).map((e) => e.id)).toEqual(['a', 'b', 'c']);
    expect(filterEntries(all, 'terminal-3', null).map((e) => e.id)).toEqual(['c']);
    expect(filterEntries(all, null, ['terminal-2', 'chat-1']).map((e) => e.id)).toEqual(['a', 'b']);
  });
});
