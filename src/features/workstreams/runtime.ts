// Connects workstreams to the panels: turn ends, permissions, errors, user typing and budget
// alerts become engine inputs; engine actions deliver handoffs through the panels' ports
// (only to members of the same workstream), glow panels and append to the workstream log.
// API managers get the manager instructions as extra system text, CLI managers a one-time
// setup message. Handoff text is data: it is typed into an agent, never run by AgentFlow.
import { createEffect, createRoot } from 'solid-js';
import { ipc } from '../../shared/ipc';
import type { HandoffEntry } from '../../shared/types';
import {
  agentState,
  onTurnEnded,
  onUserInput,
  portOf,
  requestAttention,
  setExtraSystem,
} from '../agentui';
import { reportError } from '../i18n';
import { onBudgetAlert } from '../tokens';
import { managerInstructions, setupMessage, type MemberInfo } from './messages';
import { memberName, members, type Handoff, type Workstream } from './model';
import {
  dispatch,
  handoffSent,
  setActionRunner,
  workstreamById,
  workstreamOf,
  workstreams,
} from './store';

export interface RuntimeHooks {
  panelTitle(panelId: string): string;
  /** "Claude Code", "API agent · glm-5.3", ... */
  panelKind(panelId: string): string;
  /** The focused panel while the window has focus (its turn ends need no glow). */
  isWatched(panelId: string): boolean;
  workspace(): string | null;
}

const sessionStart = Date.now();
const lastSent = new Map<string, number>();

function workers(ws: Workstream, hooks: RuntimeHooks): MemberInfo[] {
  return ws.workers.map((id) => ({ name: memberName(ws, id), kind: hooks.panelKind(id) }));
}

/** The sender's tokens since its last handoff, when the usage tracker knows them. */
async function tokensOf(panel: string): Promise<number | null> {
  try {
    const totals = await ipc.usagePanelTotals(panel, lastSent.get(panel) ?? sessionStart);
    if (totals.requests === 0) return null;
    return (
      totals.inputTokens + totals.outputTokens + totals.cacheReadTokens + totals.cacheWriteTokens
    );
  } catch {
    return null;
  }
}

async function log(ws: Workstream, handoff: Handoff, hooks: RuntimeHooks): Promise<void> {
  const tokens = await tokensOf(handoff.from);
  lastSent.set(handoff.from, Date.now());
  const entry: HandoffEntry = {
    id: handoff.id,
    workstream: ws.id,
    at: Date.now(),
    from: handoff.from,
    to: handoff.to,
    fromName: memberName(ws, handoff.from),
    toName: memberName(ws, handoff.to),
    kind: handoff.kind,
    mode: ws.mode,
    size: 0,
    tokens,
    text: handoff.kind === 'report' ? handoff.full : handoff.text,
    truncated: handoff.truncated,
  };
  await ipc.workstreamLogAppend(hooks.workspace(), entry);
}

async function deliver(ws: Workstream, handoff: Handoff, hooks: RuntimeHooks): Promise<void> {
  // Routing stays inside the workstream: never to a panel that is not a member.
  if (!members(ws).includes(handoff.to)) return;
  const port = portOf(handoff.to);
  let sent = false;
  if (port?.ready()) {
    try {
      sent = await port.deliver(handoff.text);
    } catch {
      sent = false;
    }
  }
  if (!sent) {
    dispatch(ws.id, { type: 'failed', handoff });
    return;
  }
  dispatch(ws.id, { type: 'delivered', handoff });
  handoffSent(workstreamById(ws.id) ?? ws, handoff);
  log(ws, handoff, hooks).catch((err: unknown) => reportError('error.workstreamLog', err));
}

/** Starts the runtime once (app start). Returns a cleanup function. */
export function installWorkstreams(hooks: RuntimeHooks): () => void {
  setActionRunner((ws, action) => {
    if (action.type === 'attention') {
      requestAttention(action.panel, action.reason, memberName(ws, action.panel));
    } else {
      void deliver(ws, action.handoff, hooks);
    }
  });
  const stops = [
    onTurnEnded((end) => {
      const ws = workstreamOf(end.panel);
      if (ws) dispatch(ws.id, { type: 'turn', end });
      else if (!end.interrupted && !hooks.isWatched(end.panel)) {
        requestAttention(end.panel, 'turn', hooks.panelTitle(end.panel));
      }
    }),
    onUserInput((panel) => {
      const ws = workstreamOf(panel);
      if (ws) dispatch(ws.id, { type: 'typed', panel });
    }),
    onBudgetAlert((status) => {
      if (status.level < 100) return;
      for (const ws of workstreams()) dispatch(ws.id, { type: 'budget' });
    }),
  ];
  const disposeEffects = createRoot((dispose) => {
    // Permissions and errors of members (state transitions only).
    const seen = new Map<string, string>();
    createEffect(() => {
      for (const ws of workstreams()) {
        for (const panel of members(ws)) {
          const state = agentState(panel);
          if (seen.get(panel) === state) continue;
          seen.set(panel, state);
          if (state === 'wait') dispatch(ws.id, { type: 'permission', panel });
          if (state === 'error') dispatch(ws.id, { type: 'error', panel, message: null });
        }
      }
    });
    // Manager instructions: API managers as extra system text, CLI managers once by setup
    // (when the manager's panel is open, i.e. its port registered).
    const started = new Set<string>();
    createEffect(() => {
      const managers = new Set<string>();
      for (const ws of workstreams()) {
        managers.add(ws.manager);
        const port = portOf(ws.manager);
        if (port?.kind === 'api') {
          setExtraSystem(ws.manager, managerInstructions(ws.name, workers(ws, hooks)));
        }
        if (!port || started.has(ws.id)) continue;
        started.add(ws.id);
        if (ws.reason === 'restored') continue;
        const setup = port.kind === 'cli' ? setupMessage(ws.name, workers(ws, hooks)) : null;
        const id = ws.id;
        queueMicrotask(() => dispatch(id, { type: 'start', setup }));
      }
      for (const panel of extraHolders) if (!managers.has(panel)) setExtraSystem(panel, null);
      extraHolders.clear();
      managers.forEach((panel) => extraHolders.add(panel));
    });
    return dispose;
  });
  return () => {
    stops.forEach((stop) => stop());
    disposeEffects();
  };
}

const extraHolders = new Set<string>();

/** "Stop all workstreams" (status bar). */
export function stopAll(): void {
  for (const ws of workstreams()) {
    if (ws.mode === 'auto' && ws.status !== 'done') dispatch(ws.id, { type: 'stop' });
  }
}
