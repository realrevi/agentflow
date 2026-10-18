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
  clearAttention,
  onTurnEnded,
  onUserInput,
  portOf,
  requestAttention,
  setExtraSystem,
} from '../agentui';
import { reportError } from '../i18n';
import { onBudgetAlert } from '../tokens';
import { FORMAT_REMINDER, managerInstructions, setupMessage, type MemberInfo } from './messages';
import { canRoute, memberName, members, type Handoff, type Workstream } from './model';
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
  /** The focused panel while the window has focus (its turn ends need no glow). */
  isWatched(panelId: string): boolean;
  workspace(): string | null;
}

const sessionStart = Date.now();
const lastSent = new Map<string, number>();

/** "Claude Code", "API agent (glm-5.3)" (model-facing, so not translated). */
function kindOf(panel: string): string {
  const port = portOf(panel);
  if (!port) return 'agent';
  return port.kind === 'api' ? `API agent (${port.label()})` : port.label();
}

function workers(ws: Workstream): MemberInfo[] {
  return ws.workers.map((id) => ({
    name: memberName(ws, id),
    kind: kindOf(id),
    role: portOf(id)?.role?.() ?? null,
  }));
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
  // Routing stays inside the workstream: never to or from a panel that is not a member.
  if (!canRoute(ws, handoff.from, handoff.to)) return;
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
  // The sender glowed for this handoff (a click was needed); that is done now.
  clearAttention(handoff.from);
  handoffSent(workstreamById(ws.id) ?? ws, handoff);
  log(ws, handoff, hooks).catch((err: unknown) => reportError('error.workstreamLog', err));
}

/** The one automatic format reminder after a reply without a valid block. */
async function autoRemind(ws: Workstream): Promise<void> {
  const port = portOf(ws.manager);
  let sent = false;
  try {
    sent = Boolean(port?.ready()) && (await port?.deliver(FORMAT_REMINDER)) === true;
  } catch {
    sent = false;
  }
  if (!sent) dispatch(ws.id, { type: 'error', panel: ws.manager, message: null });
}

/** Starts the runtime once (app start). Returns a cleanup function. */
export function installWorkstreams(hooks: RuntimeHooks): () => void {
  setActionRunner((ws, action) => {
    if (action.type === 'attention') {
      requestAttention(action.panel, action.reason, memberName(ws, action.panel));
    } else if (action.type === 'remind') {
      void autoRemind(ws);
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
          setExtraSystem(ws.manager, managerInstructions(ws.name, workers(ws)));
        }
        const id = ws.id;
        // A CLI manager whose process (re)started is a new session without the setup.
        const running = port?.kind === 'cli' && port.ready();
        if (running && cliWasDown.get(ws.manager)) {
          started.delete(id);
          fresh.add(id);
        }
        if (port?.kind === 'cli') cliWasDown.set(ws.manager, !running);
        if (!port || started.has(id)) continue;
        if (ws.reason === 'restored' && !fresh.has(id)) {
          started.add(id);
          continue;
        }
        if (port.kind === 'api') {
          started.add(id);
          queueMicrotask(() => dispatch(id, { type: 'start', setup: null }));
          continue;
        }
        // A CLI manager gets its setup once it runs and its startup output settled (text
        // typed into a TUI that is still drawing can be lost).
        if (!cliSettled(ws.manager) || settling.has(id)) continue;
        settling.set(
          id,
          window.setTimeout(() => {
            settling.delete(id);
            const current = workstreamById(id);
            if (!current || started.has(id) || !cliSettled(current.manager)) return;
            started.add(id);
            const again = fresh.delete(id);
            const setup = setupMessage(current.name, workers(current));
            dispatch(id, { type: 'start', setup, fresh: again });
          }, SETTLE_MS),
        );
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
const settling = new Map<string, number>();
const fresh = new Set<string>();
/** Per CLI manager panel: its process was not running when last seen. */
const cliWasDown = new Map<string, boolean>();
const SETTLE_MS = 2500;

/** Reactive: the CLI in `panel` runs and is not busy drawing or working. */
function cliSettled(panel: string): boolean {
  const state = agentState(panel);
  return Boolean(portOf(panel)?.ready()) && state !== 'work' && state !== 'think';
}

/** The user asks to remind the manager of the block format (after no or a bad block). */
export async function remindManager(id: string): Promise<void> {
  const ws = workstreamById(id);
  const port = ws ? portOf(ws.manager) : undefined;
  if (!ws || !port?.ready()) return;
  if (await port.deliver(FORMAT_REMINDER)) dispatch(id, { type: 'resume' });
}

/** "Stop all workstreams" (status bar). */
export function stopAll(): void {
  for (const ws of workstreams()) {
    if (ws.mode === 'auto' && ws.status !== 'done') dispatch(ws.id, { type: 'stop' });
  }
}
