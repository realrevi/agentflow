import { describe, expect, it } from 'vitest';
import { MAX_PROMPTS, merge } from './history';

const entry = (text: string, at = 0) => ({ text, at });

describe('prompt history merge', () => {
  it('appends in order and collapses a repeat of the last prompt', () => {
    const merged = merge([entry('a', 1), entry('b', 2)], [entry('b', 3), entry('c', 4)]);
    expect(merged).toEqual([entry('a', 1), entry('b', 3), entry('c', 4)]);
  });

  it('keeps only the newest 200 prompts', () => {
    const many = Array.from({ length: MAX_PROMPTS + 25 }, (_, i) => entry(`p${i}`, i));
    const merged = merge([], many);
    expect(merged).toHaveLength(MAX_PROMPTS);
    expect(merged[0]?.text).toBe('p25');
    expect(merged.at(-1)?.text).toBe(`p${MAX_PROMPTS + 24}`);
  });

  it('puts prompts sent before the backend answered after the loaded ones', () => {
    expect(merge([entry('old', 1)], [entry('new', 2)]).map((e) => e.text)).toEqual(['old', 'new']);
  });
});
