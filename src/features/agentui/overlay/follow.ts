// Where the corner avatar goes when it rides above the terminal cursor. Pure geometry from
// public numbers: the cursor cell, the grid size and the terminal host's box.

/** Where the cursor is, in cells of the visible screen (`row` < 0 or >= rows: scrolled away). */
export interface CursorCell {
  col: number;
  row: number;
  cols: number;
  rows: number;
}

export interface Box {
  left: number;
  top: number;
  width: number;
  height: number;
}

/**
 * Top-left of an avatar of `size` px standing just above the cursor, clamped inside
 * `bounds`. `null` when the cursor is scrolled out of view.
 */
export function avatarAboveCursor(
  cursor: CursorCell,
  host: Box,
  bounds: { width: number; height: number },
  size: number,
): { x: number; y: number } | null {
  if (cursor.row < 0 || cursor.row >= cursor.rows || cursor.cols <= 0) return null;
  const cellWidth = host.width / cursor.cols;
  const cellHeight = host.height / cursor.rows;
  const x = host.left + (cursor.col + 0.5) * cellWidth - size / 2;
  const y = host.top + cursor.row * cellHeight - size;
  return {
    x: Math.min(Math.max(0, bounds.width - size), Math.max(0, x)),
    y: Math.min(Math.max(0, bounds.height - size), Math.max(0, y)),
  };
}

/** One smoothing step towards `to`; snaps when close. */
export function lerp(from: number, to: number, amount = 0.2): number {
  const next = from + (to - from) * amount;
  return Math.abs(to - next) < 0.5 ? to : next;
}
