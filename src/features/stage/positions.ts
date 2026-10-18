// Where the user dragged characters on the Stage, per workspace and panel (in Stage
// cells, so they survive resizing). Persisted with the layout; "Auto arrange" clears a
// workspace's positions.
import { createSignal } from 'solid-js';

export interface Cell {
  x: number;
  y: number;
}

type Positions = Record<string, Record<string, Cell>>;

const [positions, setPositions] = createSignal<Positions>({});
const listeners = new Set<() => void>();

const KEY_MAX = 512;

export function workspaceKey(workspace: string | null): string {
  return (workspace ?? '_home').slice(0, KEY_MAX);
}

/** Reactive: the dragged positions of one workspace. */
export function placedCells(workspace: string | null): Record<string, Cell> {
  return positions()[workspaceKey(workspace)] ?? {};
}

function changed(): void {
  for (const listener of [...listeners]) listener();
}

export function placeCell(workspace: string | null, panel: string, cell: Cell): void {
  const key = workspaceKey(workspace);
  setPositions((all) => ({ ...all, [key]: { ...(all[key] ?? {}), [panel]: cell } }));
  changed();
}

/** "Auto arrange". */
export function clearCells(workspace: string | null): void {
  const key = workspaceKey(workspace);
  setPositions((all) => {
    const next = { ...all };
    delete next[key];
    return next;
  });
  changed();
}

export function onStageChanged(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function snapshotStage(): Record<string, unknown> {
  return { positions: positions() };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isCell(value: unknown): value is Cell {
  if (!isRecord(value)) return false;
  const { x, y } = value;
  return typeof x === 'number' && typeof y === 'number' && Number.isFinite(x) && Number.isFinite(y);
}

export function restoreStage(raw: Record<string, unknown>): void {
  const out: Positions = {};
  const saved = isRecord(raw['positions']) ? raw['positions'] : {};
  for (const [workspace, cells] of Object.entries(saved)) {
    if (!isRecord(cells)) continue;
    const valid = Object.entries(cells).filter((entry): entry is [string, Cell] =>
      isCell(entry[1]),
    );
    if (valid.length > 0) {
      out[workspace] = Object.fromEntries(
        valid.map(([panel, cell]) => [panel, { x: Math.round(cell.x), y: Math.round(cell.y) }]),
      );
    }
  }
  setPositions(out);
}
