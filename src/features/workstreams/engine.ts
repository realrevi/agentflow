// The workstream state machine, pure: an input (a turn ended, a permission, the user typed,
// Stop, a click on Send, ...) and the current workstream give the next workstream and the
// actions to run (deliver a handoff, glow a panel). Modes:
//   manual / semiAuto: every handoff waits as an editable preview (manual adds the glow and
//   buttons, semi-auto shows the card at once); one click sends it.
//   auto: handoffs go at once until a stop condition: an ask or done, any pending
//   permission (never answered for the user), an error or malformed reply, max rounds, a
//   budget hit, the user typing into a panel of the workstream, or Stop.
import type { HandoffMode } from '../../shared/types';
import type { TurnEnd } from '../agentui/turns';
import { reportMessage } from './messages';
import { memberName, members, type Handoff, type PauseReason, type Workstream } from './model';
import { parseReply, resolveWorker } from './protocol';

export type Input =
  | { type: 'turn'; end: TurnEnd }
  | { type: 'start'; setup: string | null }
  | { type: 'permission'; panel: string }
  | { type: 'error'; panel: string; message: string | null }
  | { type: 'typed'; panel: string }
  | { type: 'budget' }
  | { type: 'stop' }
  | { type: 'resume' }
  | { type: 'send'; text?: string }
  | { type: 'dismiss' }
  | { type: 'mode'; mode: HandoffMode }
  | { type: 'delivered'; handoff: Handoff }
  | { type: 'failed'; handoff: Handoff };

export type Action =
  | { type: 'deliver'; handoff: Handoff }
  | { type: 'attention'; panel: string; reason: 'turn' | 'question' | 'error' };

export interface Limits {
  maxRounds: number;
  /** Report cap in bytes. */
  reportCap: number;
}

export interface Step {
  ws: Workstream;
  actions: Action[];
}

type NewId = () => string;

function pause(ws: Workstream, reason: PauseReason, detail: string | null = null): Workstream {
  return { ...ws, status: reason === 'done' ? 'done' : 'paused', reason, detail };
}

const isRunningAuto = (ws: Workstream) => ws.mode === 'auto' && ws.status === 'running';

/** A handoff is ready: sent now (auto, running) or held as a preview. */
function offer(ws: Workstream, handoff: Handoff, limits: Limits): Step {
  const autoOk = ws.mode === 'auto' && ws.status !== 'paused';
  if (autoOk && handoff.kind === 'prompt' && ws.round >= limits.maxRounds) {
    return {
      ws: { ...pause(ws, 'maxRounds'), pending: handoff },
      actions: [{ type: 'attention', panel: ws.manager, reason: 'question' }],
    };
  }
  if (autoOk) return deliver({ ...ws, pending: null }, handoff);
  const status =
    ws.status === 'paused'
      ? ws
      : { ...ws, status: 'waiting' as const, reason: 'review' as const, detail: null };
  return {
    ws: { ...status, pending: handoff },
    actions: [{ type: 'attention', panel: handoff.from, reason: 'turn' }],
  };
}

function deliver(ws: Workstream, handoff: Handoff): Step {
  const round = handoff.kind === 'prompt' ? ws.round + 1 : ws.round;
  const awaiting = [...ws.awaiting.filter((p) => p !== handoff.to), handoff.to];
  return {
    ws: { ...ws, status: 'running', reason: null, detail: null, round, awaiting, pending: null },
    actions: [{ type: 'deliver', handoff }],
  };
}

function managerTurn(ws: Workstream, end: TurnEnd, limits: Limits, newId: NewId): Step {
  const ask = (reason: PauseReason, detail: string | null, glow: 'question' | 'error'): Step => ({
    ws: { ...pause(ws, reason, detail), warning: null },
    actions: [{ type: 'attention', panel: ws.manager, reason: glow }],
  });
  if (end.interrupted) return ask('interrupted', null, 'question');
  // The user answered a question or gave a new task: a new auto run.
  if (ws.reason === 'ask' || ws.status === 'done') ws = { ...ws, round: 0 };
  const parsed = parseReply(end.report);
  if (!parsed.ok) {
    return parsed.error === 'none'
      ? ask('noBlock', null, 'question')
      : ask('malformed', parsed.detail, 'error');
  }
  const warning = parsed.multiple ? 'multiple' : null;
  const directive = parsed.directive;
  if (directive.kind === 'ask') {
    return {
      ws: { ...ws, status: 'waiting', reason: 'ask', detail: directive.body, warning },
      actions: [{ type: 'attention', panel: ws.manager, reason: 'question' }],
    };
  }
  if (directive.kind === 'done') {
    return {
      ws: { ...pause(ws, 'done', directive.body), warning, awaiting: [] },
      actions: [{ type: 'attention', panel: ws.manager, reason: 'turn' }],
    };
  }
  const workers = ws.workers.map((id) => ({ id, name: memberName(ws, id) }));
  const to = resolveWorker(directive.to, workers);
  if (!to) return ask('unknownWorker', directive.to, 'error');
  const handoff: Handoff = {
    id: newId(),
    from: ws.manager,
    to,
    kind: 'prompt',
    text: directive.body,
    full: directive.body,
    truncated: false,
  };
  return offer({ ...ws, warning }, handoff, limits);
}

function workerTurn(ws: Workstream, end: TurnEnd, limits: Limits, newId: NewId): Step {
  const awaiting = ws.awaiting.filter((p) => p !== end.panel);
  const next = { ...ws, awaiting };
  if (end.interrupted) {
    return {
      ws: pause(next, 'interrupted', memberName(ws, end.panel)),
      actions: [{ type: 'attention', panel: end.panel, reason: 'question' }],
    };
  }
  const report = reportMessage(memberName(ws, end.panel), end.turn, end.report, limits.reportCap);
  const handoff: Handoff = {
    id: newId(),
    from: end.panel,
    to: ws.manager,
    kind: 'report',
    text: report.text,
    full: report.full,
    truncated: report.truncated,
  };
  return offer(next, handoff, limits);
}

/** Auto mode only: these stop a running workstream. */
function autoStop(ws: Workstream, reason: PauseReason, detail: string | null = null): Step {
  return { ws: isRunningAuto(ws) ? pause(ws, reason, detail) : ws, actions: [] };
}

export function step(ws: Workstream, input: Input, limits: Limits, newId: NewId): Step {
  const isMember = (panel: string) => members(ws).includes(panel);
  switch (input.type) {
    case 'turn': {
      const { end } = input;
      if (end.panel === ws.manager) return managerTurn(ws, end, limits, newId);
      if (ws.workers.includes(end.panel) && ws.awaiting.includes(end.panel)) {
        return workerTurn(ws, end, limits, newId);
      }
      // A worker's own conversation with the user: not part of the workstream.
      return { ws, actions: [] };
    }
    case 'start': {
      if (ws.setupSent || !input.setup)
        return { ws: { ...ws, status: 'idle', reason: null }, actions: [] };
      const handoff: Handoff = {
        id: newId(),
        from: ws.manager,
        to: ws.manager,
        kind: 'setup',
        text: input.setup,
        full: input.setup,
        truncated: false,
      };
      return offer({ ...ws, status: 'idle' }, handoff, limits);
    }
    case 'permission':
      return isMember(input.panel) ? autoStop(ws, 'permission') : { ws, actions: [] };
    case 'error':
      return isMember(input.panel) ? autoStop(ws, 'error', input.message) : { ws, actions: [] };
    case 'typed':
      return isMember(input.panel) ? autoStop(ws, 'userTyped') : { ws, actions: [] };
    case 'budget':
      return autoStop(ws, 'budget');
    case 'stop':
      return {
        ws: ws.status === 'done' ? ws : pause(ws, 'stopped'),
        actions: [],
      };
    case 'resume': {
      const resumed: Workstream = {
        ...ws,
        status: 'running',
        reason: null,
        detail: null,
        round: 0,
      };
      if (ws.pending && ws.mode === 'auto') return deliver(resumed, ws.pending);
      return {
        ws: ws.pending ? { ...resumed, status: 'waiting', reason: 'review' } : resumed,
        actions: [],
      };
    }
    case 'send': {
      if (!ws.pending) return { ws, actions: [] };
      const text = input.text?.trim() ? input.text : ws.pending.text;
      const fresh = ws.reason === 'maxRounds' ? { ...ws, round: 0 } : ws;
      return deliver(fresh, { ...ws.pending, text });
    }
    case 'dismiss':
      return {
        ws: { ...ws, pending: null, status: 'idle', reason: null, detail: null },
        actions: [],
      };
    case 'mode':
      return { ws: { ...ws, mode: input.mode }, actions: [] };
    case 'delivered':
      return {
        ws: input.handoff.kind === 'setup' ? { ...ws, setupSent: true } : ws,
        actions: [],
      };
    case 'failed':
      return {
        ws: {
          ...pause(ws, 'notRunning', memberName(ws, input.handoff.to)),
          pending: input.handoff,
          awaiting: ws.awaiting.filter((p) => p !== input.handoff.to),
          round: input.handoff.kind === 'prompt' ? Math.max(0, ws.round - 1) : ws.round,
        },
        actions: [{ type: 'attention', panel: input.handoff.to, reason: 'error' }],
      };
  }
}
