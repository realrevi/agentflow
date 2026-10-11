// Where the caret is in a monospace, soft-wrapping textarea (white-space: pre-wrap;
// word-break: break-all), in character cells. The walking avatar follows it.

/** East Asian wide and fullwidth characters take two cells. */
export function cellWidth(code: number): 1 | 2 {
  const wide =
    (code >= 0x1100 && code <= 0x115f) ||
    (code >= 0x2e80 && code <= 0xa4cf) ||
    (code >= 0xac00 && code <= 0xd7a3) ||
    (code >= 0xf900 && code <= 0xfaff) ||
    (code >= 0xfe30 && code <= 0xfe4f) ||
    (code >= 0xff00 && code <= 0xff60) ||
    (code >= 0xffe0 && code <= 0xffe6) ||
    (code >= 0x1f300 && code <= 0x1faff) ||
    (code >= 0x20000 && code <= 0x3fffd);
  return wide ? 2 : 1;
}

export interface CaretCell {
  col: number;
  row: number;
}

/**
 * The caret's cell after `caret` UTF-16 units of `text`, wrapping at `columns` cells. A
 * character that does not fit on the current line moves to the next one.
 */
export function caretCell(text: string, caret: number, columns: number): CaretCell {
  const width = Math.max(1, Math.floor(columns));
  let col = 0;
  let row = 0;
  for (const char of text.slice(0, Math.max(0, caret))) {
    if (char === '\n') {
      row++;
      col = 0;
      continue;
    }
    const cells = cellWidth(char.codePointAt(0) ?? 0);
    if (col + cells > width) {
      row++;
      col = 0;
    }
    col += cells;
  }
  return { col, row };
}

export interface CaretMetrics {
  charWidth: number;
  lineHeight: number;
  paddingLeft: number;
  paddingTop: number;
  scrollTop: number;
}

/** Pixel position (relative to the textarea's border box) of the top of the caret. */
export function caretPixel(cell: CaretCell, metrics: CaretMetrics): { x: number; y: number } {
  return {
    x: metrics.paddingLeft + cell.col * metrics.charWidth,
    y: metrics.paddingTop + cell.row * metrics.lineHeight - metrics.scrollTop,
  };
}
