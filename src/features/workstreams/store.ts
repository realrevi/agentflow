// All workstreams of the layout: create, link and unlink panels, remove, and `dispatch`,
// which runs an input through the engine and hands the resulting actions to the runtime.
// Changes are announced for persisting with the layout; handoffs for the Stage.
import { createStore, produce, reconcile, unwrap } from 'solid-js/store';
import type { HandoffMode } from '../../shared/types';
import { settings } from '../settings/settingsStore';
import { step, type Action, type Input } from './engine';
import {
  linkedElsewhere,
  members,
  parseWorkstream,
  serializeWorkstream,
  type Handoff,
  type Workstream,
} from './model';

const [state, setState] = createStore<{ list: Workstream[] }>({ list: [] });

type ActionRunner = (ws: Workstream, action: Action) => void;
let runAction: ActionRunner = () => undefined;
const changeListeners = new Set<() => void>();
const handoffListeners = new Set<(ws: Workstream, handoff: Handoff) => void>();

/** Reactive list. */
export function workstreams(): readonly Workstream[] {
  return state.list;
}

export function workstreamById(id: string): Workstream | undefined {
  return state.list.find((ws) => ws.id === id);
}

/** The workstream a panel belongs to (as manager or worker). Reactive. */
export function workstreamOf(panelId: string): Workstream | undefined {
  return state.list.find((ws) => members(ws).includes(panelId));
}

export function anyAutoRunning(): boolean {
  return state.list.some((ws) => ws.mode === 'auto' && ws.status === 'running');
}

/** The runtime executes engine actions (deliveries, glows). */
export function setActionRunner(runner: ActionRunner): void {
  runAction = runner;
}

export function onWorkstreamsChanged(listener: () => void): () => void {
  changeListeners.add(listener);
  return () => {
    changeListeners.delete(listener);
  };
}

/** A handoff was delivered (the Stage flies a packet). */
export function onHandoffSent(listener: (ws: Workstream, handoff: Handoff) => void): () => void {
  handoffListeners.add(listener);
  return () => {
    handoffListeners.delete(listener);
  };
}

export function handoffSent(ws: Workstream, handoff: Handoff): void {
  for (const listener of [...handoffListeners]) listener(ws, handoff);
}

function changed(): void {
  for (const listener of [...changeListeners]) listener();
}

function replace(next: Workstream): void {
  setState('list', (ws) => ws.id === next.id, reconcile(next));
  changed();
}

export function limits() {
  return {
    maxRounds: settings.workstreams.maxRounds,
    reportCap: settings.workstreams.reportCapKb * 1024,
  };
}

/** Runs `input` through the engine for workstream `id`, then its actions. */
export function dispatch(id: string, input: Input): void {
  const current = workstreamById(id);
  if (!current) return;
  const result = step(structuredClone(unwrap(current)), input, limits(), () => crypto.randomUUID());
  replace(result.ws);
  for (const action of result.actions) runAction(result.ws, action);
}

export interface NewWorkstream {
  name: string;
  color: string;
  mode: HandoffMode;
  manager: string;
  workers: string[];
  names: Record<string, string>;
}

/** Creates a workstream; panels already in another one are left out. Returns its id. */
export function createWorkstream(spec: NewWorkstream): string {
  const id = `ws-${crypto.randomUUID().slice(0, 8)}`;
  const free = (panel: string) => !linkedElsewhere(state.list, panel);
  const ws: Workstream = {
    id,
    name: spec.name.trim() || id,
    color: spec.color,
    mode: spec.mode,
    manager: spec.manager,
    workers: [...new Set(spec.workers)].filter((p) => p !== spec.manager && free(p)),
    names: spec.names,
    status: 'idle',
    reason: null,
    detail: null,
    warning: null,
    round: 0,
    setupSent: false,
    pending: null,
    awaiting: [],
  };
  setState('list', (list) => [...list, ws]);
  changed();
  return id;
}

/** Adds a worker; false if the panel is already in a workstream. */
export function linkWorker(id: string, panel: string, name: string): boolean {
  if (linkedElsewhere(state.list, panel) || !workstreamById(id)) return false;
  setState(
    'list',
    (ws) => ws.id === id,
    produce((ws) => {
      ws.workers.push(panel);
      ws.names[panel] = name;
    }),
  );
  changed();
  return true;
}

/** Takes a panel out of its workstream; a manager leaving removes the workstream. */
export function unlinkPanel(panel: string): void {
  const ws = workstreamOf(panel);
  if (!ws) return;
  if (ws.manager === panel) {
    removeWorkstream(ws.id);
    return;
  }
  setState(
    'list',
    (item) => item.id === ws.id,
    produce((item) => {
      item.workers = item.workers.filter((p) => p !== panel);
      item.awaiting = item.awaiting.filter((p) => p !== panel);
      delete item.names[panel];
      if (item.pending && (item.pending.to === panel || item.pending.from === panel)) {
        item.pending = null;
      }
    }),
  );
  changed();
}

export function removeWorkstream(id: string): void {
  setState('list', (list) => list.filter((ws) => ws.id !== id));
  changed();
}

/** Edits the editable text of the pending handoff (previews). */
export function setPendingText(id: string, text: string): void {
  setState('list', (ws) => ws.id === id && ws.pending !== null, 'pending', 'text', text);
}

/** For the saved layout. */
export function snapshotWorkstreams(): Record<string, unknown>[] {
  return state.list.map((ws) => serializeWorkstream(unwrap(ws)));
}

/** From the saved layout; panels that no longer exist are dropped by the layout. */
export function restoreWorkstreams(raw: unknown, panelExists: (id: string) => boolean): void {
  const list = (Array.isArray(raw) ? raw : [])
    .map(parseWorkstream)
    .filter((ws): ws is Workstream => ws !== null && panelExists(ws.manager))
    .map((ws) => ({ ...ws, workers: ws.workers.filter(panelExists) }));
  // A panel belongs to one workstream at most (the first one wins).
  const seen = new Set<string>();
  const valid = list.filter((ws) => {
    const own = members(ws);
    if (own.some((panel) => seen.has(panel))) return false;
    own.forEach((panel) => seen.add(panel));
    return true;
  });
  setState('list', valid);
}
