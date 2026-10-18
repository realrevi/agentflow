// A workstream: one manager panel and 1..n worker panels (any agent panel kind), with a name,
// a color and a handoff mode. Persisted with the layout (only what survives a restart:
// membership, names, mode, round, a pending handoff); parsed defensively on restore.
import type { HandoffMode } from '../../shared/types';

export type WsStatus = 'idle' | 'running' | 'waiting' | 'paused' | 'done';

export type PauseReason =
  | 'ask'
  | 'done'
  | 'review'
  | 'permission'
  | 'error'
  | 'malformed'
  | 'noBlock'
  | 'unknownWorker'
  | 'maxRounds'
  | 'budget'
  | 'userTyped'
  | 'stopped'
  | 'notRunning'
  | 'interrupted'
  | 'restored';

export type HandoffKind = 'prompt' | 'report' | 'setup';

export interface Handoff {
  id: string;
  /** Panel ids (a setup goes from the manager to itself). */
  from: string;
  to: string;
  kind: HandoffKind;
  /** What will be sent (editable in previews). */
  text: string;
  /** The whole text (a report before the cap). */
  full: string;
  truncated: boolean;
}

export interface Workstream {
  id: string;
  name: string;
  /** Hex color of its label and lines. */
  color: string;
  mode: HandoffMode;
  manager: string;
  workers: string[];
  /** Protocol names by panel id (the manager sends `to="<name>"`). */
  names: Record<string, string>;
  status: WsStatus;
  reason: PauseReason | null;
  /** e.g. the unknown worker's name, an error message. */
  detail: string | null;
  /** A non-blocking note (several blocks in one reply). */
  warning: string | null;
  /** Manager -> worker handoffs in the current auto run. */
  round: number;
  setupSent: boolean;
  pending: Handoff | null;
  /** Panels whose next turn end answers a delivery. */
  awaiting: string[];
}

export const WORKSTREAM_COLORS = [
  '#F5C542',
  '#5EC8E5',
  '#B48EF0',
  '#7BD88F',
  '#F08A8A',
  '#F2A65A',
] as const;

const MODES: readonly HandoffMode[] = ['manual', 'semiAuto', 'auto'];
const HEX = /^#[0-9a-fA-F]{6}$/;
const ID = /^[A-Za-z0-9_-]{1,64}$/;
const MAX_NAME = 60;

export function members(ws: Workstream): string[] {
  return [ws.manager, ...ws.workers];
}

export function memberName(ws: Workstream, panel: string): string {
  return ws.names[panel] ?? panel;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function text(value: unknown, max: number): string | null {
  return typeof value === 'string' && value.trim() ? value.trim().slice(0, max) : null;
}

function parseHandoff(raw: unknown): Handoff | null {
  if (!isRecord(raw)) return null;
  const { id, from, to, kind, text: body, full, truncated } = raw;
  if (typeof id !== 'string' || typeof from !== 'string' || typeof to !== 'string') return null;
  if (kind !== 'prompt' && kind !== 'report' && kind !== 'setup') return null;
  if (typeof body !== 'string') return null;
  return {
    id,
    from,
    to,
    kind,
    text: body,
    full: typeof full === 'string' ? full : body,
    truncated: truncated === true,
  };
}

/** One persisted workstream, or null if it is not usable. Restored ones start paused
 * (agent CLIs never restart by themselves). */
export function parseWorkstream(raw: unknown): Workstream | null {
  if (!isRecord(raw)) return null;
  const id = typeof raw['id'] === 'string' && ID.test(raw['id']) ? raw['id'] : null;
  const name = text(raw['name'], MAX_NAME);
  const manager = typeof raw['manager'] === 'string' ? raw['manager'] : null;
  if (!id || !name || !manager) return null;
  const workers = Array.isArray(raw['workers'])
    ? raw['workers'].filter((w): w is string => typeof w === 'string' && w !== manager)
    : [];
  const names: Record<string, string> = {};
  if (isRecord(raw['names'])) {
    for (const [panel, value] of Object.entries(raw['names'])) {
      const label = text(value, MAX_NAME);
      if (label) names[panel] = label;
    }
  }
  const color = typeof raw['color'] === 'string' && HEX.test(raw['color']) ? raw['color'] : null;
  const mode = MODES.find((m) => m === raw['mode']) ?? 'semiAuto';
  const round =
    typeof raw['round'] === 'number' && raw['round'] >= 0 ? Math.floor(raw['round']) : 0;
  const pending = parseHandoff(raw['pending']);
  return {
    id,
    name,
    color: color ?? WORKSTREAM_COLORS[0],
    mode,
    manager,
    workers: [...new Set(workers)],
    names,
    status: 'paused',
    reason: 'restored',
    detail: null,
    warning: null,
    round,
    setupSent: raw['setupSent'] === true,
    pending,
    awaiting: [],
  };
}

/** What is persisted. */
export function serializeWorkstream(ws: Workstream): Record<string, unknown> {
  return {
    id: ws.id,
    name: ws.name,
    color: ws.color,
    mode: ws.mode,
    manager: ws.manager,
    workers: ws.workers,
    names: ws.names,
    round: ws.round,
    setupSent: ws.setupSent,
    pending: ws.pending,
  };
}

/** Workers keep at most one workstream; the manager may not be a worker anywhere. */
export function linkedElsewhere(all: readonly Workstream[], panel: string, except?: string) {
  return all.find((ws) => ws.id !== except && members(ws).includes(panel)) ?? null;
}
