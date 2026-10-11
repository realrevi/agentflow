import { describe, expect, it } from 'vitest';
import { merge } from './history';
import { arc } from './packet';
import { FRESH, newer, older } from './recall';

describe('prompt recall', () => {
  const history = ['first', 'second', 'third'];

  it('walks back and forth and restores the draft', () => {
    let step = older(history, FRESH, 'my draft');
    expect(step?.text).toBe('third');
    step = older(history, step?.recall ?? FRESH, 'third');
    expect(step?.text).toBe('second');
    step = newer(history, step?.recall ?? FRESH);
    expect(step?.text).toBe('third');
    step = newer(history, step?.recall ?? FRESH);
    expect(step).toEqual({ text: 'my draft', recall: FRESH });
    expect(newer(history, FRESH)).toBeNull();
    expect(older([], FRESH, '')).toBeNull();
  });

  it('stops at the oldest prompt', () => {
    const oldest = older(history, { index: 0, draft: '' }, 'first');
    expect(oldest?.text).toBe('first');
  });
});

describe('local history merge', () => {
  it('collapses repeats and keeps the last 200', () => {
    const merged = merge(
      [{ text: 'a', at: 1 }],
      [
        { text: 'a', at: 2 },
        { text: 'b', at: 3 },
      ],
    );
    expect(merged).toEqual([
      { text: 'a', at: 2 },
      { text: 'b', at: 3 },
    ]);
    const many = Array.from({ length: 250 }, (_, i) => ({ text: `p${i}`, at: i }));
    expect(merge([], many)).toHaveLength(200);
  });
});

describe('packet arc', () => {
  it('starts and ends at the given points and rises above both', () => {
    const points = arc({ x: 0, y: 100 }, { x: 200, y: 20 }, -20);
    expect(points[0]).toEqual({ x: 0, y: 100 });
    expect(points[points.length - 1]).toEqual({ x: 200, y: 20 });
    expect(Math.min(...points.map((p) => p.y))).toBeLessThan(20);
  });
});
