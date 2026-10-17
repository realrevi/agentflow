import { describe, expect, it } from 'vitest';
import type { Estimate } from '../../shared/types';
import { createAutoCompact } from './autoCompact';

const at = (share: number, auto = true): Estimate => ({
  tokens: Math.round(share * 1000),
  approximate: true,
  cost: null,
  contextWindow: 1000,
  windowSource: 'default',
  share,
  threshold: 0.7,
  shouldCompact: auto && share >= 0.7,
});

describe('auto-compact', () => {
  it('fires once per threshold crossing', () => {
    const auto = createAutoCompact();
    expect(auto.observe(at(0.5))).toBe(false);
    expect(auto.observe(at(0.72))).toBe(true);
    expect(auto.observe(at(0.75)), 'still above: no second compact').toBe(false);
    expect(auto.observe(at(0.8))).toBe(false);
    expect(auto.observe(at(0.3)), 'the compact worked').toBe(false);
    expect(auto.observe(at(0.71)), 'a new crossing').toBe(true);
  });

  it('never fires with auto-compact off or without an estimate', () => {
    const auto = createAutoCompact();
    expect(auto.observe(null)).toBe(false);
    expect(auto.observe(at(0.9, false))).toBe(false);
    // Turning it on while above the threshold still counts as this crossing.
    expect(auto.observe(at(0.9, true))).toBe(true);
    expect(auto.observe(at(0.9, true))).toBe(false);
  });
});
