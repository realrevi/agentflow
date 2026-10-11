// Splits a quick open result into file name and folder, with matched characters marked.

export interface Segment {
  text: string;
  match: boolean;
}

/** `positions` are code point indices into `path` (as sent by the backend). */
export function highlight(
  path: string,
  positions: readonly number[],
): {
  name: Segment[];
  folder: Segment[];
} {
  const chars = Array.from(path);
  const slash = chars.lastIndexOf('/');
  const marked = new Set(positions);
  const segments = (from: number, to: number): Segment[] => {
    const result: Segment[] = [];
    for (let i = from; i < to; i++) {
      const match = marked.has(i);
      const last = result[result.length - 1];
      if (last && last.match === match) last.text += chars[i];
      else result.push({ text: chars[i] ?? '', match });
    }
    return result;
  };
  return { name: segments(slash + 1, chars.length), folder: segments(0, Math.max(slash, 0)) };
}
