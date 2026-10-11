import { describe, expect, it } from 'vitest';
import { detectLineSeparator, minimalChange } from './text';

/** Applies a change to editor text (separators as one `\n`). */
function apply(editorText: string, change: ReturnType<typeof minimalChange>, separator: string) {
  if (!change) return editorText;
  const insert = change.insert.split(separator).join('\n');
  return editorText.slice(0, change.from) + insert + editorText.slice(change.to);
}

describe('line separators', () => {
  it('keeps the majority line ending', () => {
    expect(detectLineSeparator('a\nb\n')).toBe('\n');
    expect(detectLineSeparator('a\r\nb\r\n')).toBe('\r\n');
    expect(detectLineSeparator('a\r\nb\nc\n')).toBe('\n');
    expect(detectLineSeparator('no newline')).toBe('\n');
  });
});

describe('minimalChange', () => {
  it('replaces only what changed', () => {
    expect(minimalChange('hello world', 'hello brave world', '\n')).toEqual({
      from: 6,
      to: 6,
      insert: 'brave ',
    });
    expect(minimalChange('same', 'same', '\n')).toBeNull();
    expect(minimalChange('abc', '', '\n')).toEqual({ from: 0, to: 3, insert: '' });
  });

  it('maps CRLF files to editor positions without splitting pairs', () => {
    const before = 'one\r\ntwo\r\nthree\r\n';
    const after = 'one\r\nTWO\r\nthree\r\nfour\r\n';
    const change = minimalChange(before, after, '\r\n');
    const editorBefore = 'one\ntwo\nthree\n';
    expect(apply(editorBefore, change, '\r\n')).toBe('one\nTWO\nthree\nfour\n');

    const appended = minimalChange('a\r\n', 'a\r\n\r\n', '\r\n');
    expect(apply('a\n', appended, '\r\n')).toBe('a\n\n');
  });
});
