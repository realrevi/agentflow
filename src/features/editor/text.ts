// Text helpers for keeping files byte-identical: line endings and minimal reloads.

export type LineSeparator = '\n' | '\r\n';

/** The separator to keep when saving: CRLF if the file mostly uses it, else LF. */
export function detectLineSeparator(text: string): LineSeparator {
  let crlf = 0;
  let lf = 0;
  for (let i = text.indexOf('\n'); i >= 0; i = text.indexOf('\n', i + 1)) {
    if (i > 0 && text.charCodeAt(i - 1) === 13) crlf++;
    else lf++;
  }
  return crlf > lf ? '\r\n' : '\n';
}

export interface TextChange {
  from: number;
  to: number;
  insert: string;
}

/**
 * The smallest single replacement turning `before` into `after` (both as stored on disk),
 * in editor positions, where each `separator` counts as one character. `null` if equal.
 * Reloading through this keeps the cursor and scroll position where they were.
 */
export function minimalChange(
  before: string,
  after: string,
  separator: LineSeparator,
): TextChange | null {
  if (before === after) return null;
  const limit = Math.min(before.length, after.length);
  let start = 0;
  while (start < limit && before.charCodeAt(start) === after.charCodeAt(start)) start++;
  let end = 0;
  while (
    end < limit - start &&
    before.charCodeAt(before.length - 1 - end) === after.charCodeAt(after.length - 1 - end)
  ) {
    end++;
  }
  if (separator === '\r\n') {
    // Never split a CRLF pair: move the boundaries outwards.
    if (start > 0 && before.charCodeAt(start - 1) === 13) start--;
    if (end > 0 && before.charCodeAt(before.length - end) === 10) end--;
  }
  const toEditor = (position: number) =>
    separator === '\n' ? position : position - countOf(before, '\r\n', position);
  return {
    from: toEditor(start),
    to: toEditor(before.length - end),
    insert: after.slice(start, after.length - end),
  };
}

function countOf(text: string, needle: string, until: number): number {
  let count = 0;
  for (let i = text.indexOf(needle); i >= 0 && i < until; i = text.indexOf(needle, i + 1)) {
    count++;
  }
  return count;
}
