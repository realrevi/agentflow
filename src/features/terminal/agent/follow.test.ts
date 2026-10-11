import { describe, expect, it } from 'vitest';
import { avatarAboveCursor, lerp } from './follow';

const host = { left: 10, top: 6, width: 800, height: 480 };
const bounds = { width: 820, height: 500 };

describe('avatarAboveCursor', () => {
  it('stands above the cursor cell', () => {
    const cursor = { col: 10, row: 5, cols: 80, rows: 24 };
    // cell 10x20: x = 10 + 10.5 * 10 - 24, y = 6 + 5 * 20 - 48
    expect(avatarAboveCursor(cursor, host, bounds, 48)).toEqual({ x: 91, y: 58 });
  });

  it('stays inside the panel', () => {
    expect(avatarAboveCursor({ col: 0, row: 0, cols: 80, rows: 24 }, host, bounds, 48)).toEqual({
      x: 0,
      y: 0,
    });
    expect(avatarAboveCursor({ col: 80, row: 23, cols: 80, rows: 24 }, host, bounds, 48)?.x).toBe(
      772,
    );
  });

  it('hides when the cursor is scrolled out of view', () => {
    expect(avatarAboveCursor({ col: 3, row: 30, cols: 80, rows: 24 }, host, bounds, 48)).toBeNull();
    expect(avatarAboveCursor({ col: 3, row: -1, cols: 80, rows: 24 }, host, bounds, 48)).toBeNull();
  });
});

describe('lerp', () => {
  it('eases and snaps', () => {
    expect(lerp(0, 100)).toBe(20);
    expect(lerp(99.6, 100)).toBe(100);
  });
});
