import { describe, expect, it } from 'vitest';
import { caretCell, caretPixel, cellWidth } from './caret';

describe('caretCell', () => {
  it('counts columns and explicit lines', () => {
    expect(caretCell('', 0, 40)).toEqual({ col: 0, row: 0 });
    expect(caretCell('hello', 5, 40)).toEqual({ col: 5, row: 0 });
    expect(caretCell('hello', 2, 40)).toEqual({ col: 2, row: 0 });
    expect(caretCell('one\ntwo\nthree', 9, 40)).toEqual({ col: 1, row: 2 });
    expect(caretCell('one\n', 4, 40)).toEqual({ col: 0, row: 1 });
  });

  it('soft-wraps at the column count', () => {
    expect(caretCell('abcdefghij', 10, 4)).toEqual({ col: 2, row: 2 });
    expect(caretCell('abcd', 4, 4)).toEqual({ col: 4, row: 0 });
    expect(caretCell('abcde', 5, 4)).toEqual({ col: 1, row: 1 });
  });

  it('counts wide characters as two cells', () => {
    expect(cellWidth('智'.codePointAt(0) ?? 0)).toBe(2);
    expect(cellWidth('a'.codePointAt(0) ?? 0)).toBe(1);
    expect(caretCell('智能体', 3, 40)).toEqual({ col: 6, row: 0 });
    expect(caretCell('ab智', 3, 3)).toEqual({ col: 2, row: 1 });
  });

  it('turns cells into pixels', () => {
    const metrics = { charWidth: 8, lineHeight: 20, paddingLeft: 10, paddingTop: 6, scrollTop: 20 };
    expect(caretPixel({ col: 3, row: 2 }, metrics)).toEqual({ x: 34, y: 26 });
  });
});
