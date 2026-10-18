// Where characters stand on the Stage, in grid cells (one cell = one character with room
// around it). Each workstream is a cluster: its name on top, the manager below it, the
// workers in a row underneath; clusters flow left to right and wrap. Unlinked agents stand
// in a row after them. Dragged positions (per workspace) win over the auto layout.
import type { Cell } from './positions';

export interface ClusterInput {
  id: string;
  name: string;
  color: string;
  manager: string;
  workers: readonly string[];
}

export interface ClusterLabel {
  id: string;
  name: string;
  color: string;
  manager: string;
  /** Center of the label. */
  cell: Cell;
}

export interface Placement {
  cells: Map<string, Cell>;
  labels: ClusterLabel[];
}

/** Cells between clusters, and the rows one band of clusters takes. */
const GAP = 1;
const BAND = 5;
const MANAGER_ROW = 1;
const WORKER_ROW = 3;

export function autoLayout(
  clusters: readonly ClusterInput[],
  loose: readonly string[],
  maxCols: number,
): Placement {
  const cells = new Map<string, Cell>();
  const labels: ClusterLabel[] = [];
  const width = Math.max(1, Math.floor(maxCols));
  let col = 0;
  let band = 0;
  let used = false;
  for (const cluster of clusters) {
    const span = Math.max(1, cluster.workers.length);
    if (used && col + span > width) {
      col = 0;
      band += BAND;
    }
    const center = col + (span - 1) / 2;
    const { id, name, color, manager } = cluster;
    labels.push({ id, name, color, manager, cell: { x: center, y: band } });
    cells.set(cluster.manager, { x: center, y: band + MANAGER_ROW });
    cluster.workers.forEach((worker, index) => {
      cells.set(worker, { x: col + index, y: band + WORKER_ROW });
    });
    col += span + GAP;
    used = true;
  }
  const rest = loose.filter((panel) => !cells.has(panel));
  if (rest.length > 0) {
    let row = used ? band + BAND : 0;
    col = 0;
    for (const panel of rest) {
      if (col >= width) {
        col = 0;
        row += 2;
      }
      cells.set(panel, { x: col, y: row + MANAGER_ROW });
      col += 1;
    }
  }
  return { cells, labels };
}

/** The auto layout with dragged cells applied (only for panels that are on the Stage). */
export function withPlaced(
  placement: Placement,
  placed: Readonly<Record<string, Cell>>,
): Placement {
  const cells = new Map(placement.cells);
  for (const [panel, cell] of Object.entries(placed)) {
    if (cells.has(panel)) cells.set(panel, cell);
  }
  // A cluster's label follows its manager.
  const labels = placement.labels.map((label) => {
    const at = cells.get(label.manager);
    return at ? { ...label, cell: { x: at.x, y: at.y - MANAGER_ROW } } : label;
  });
  return { cells, labels };
}

/** Pixels per cell, across and down (rows shrink so a short Stage still fits). */
export interface Units {
  x: number;
  y: number;
}

/** Rows the placement uses (its lowest cell, plus room for the name under it). */
export function rowsOf(placement: Placement): number {
  let rows = 1;
  for (const cell of placement.cells.values()) rows = Math.max(rows, cell.y + 1.6);
  return rows;
}

/** Cell sizes for a Stage `height` px high: rows never get shorter than `minY`. */
export function unitsFor(base: number, minY: number, height: number, rows: number): Units {
  return { x: base, y: Math.max(minY, Math.min(base, height / Math.max(1, rows))) };
}

/** A pixel point back to the nearest half cell (dragging snaps softly). */
export function cellAt(x: number, y: number, units: Units, origin: Cell): Cell {
  const snap = (value: number) => Math.round(value * 2) / 2;
  return { x: snap((x - origin.x) / units.x - 0.5), y: snap((y - origin.y) / units.y - 0.5) };
}

/** The pixel center of a cell. */
export function cellCenter(cell: Cell, units: Units, origin: Cell): Cell {
  return { x: origin.x + (cell.x + 0.5) * units.x, y: origin.y + (cell.y + 0.5) * units.y };
}
