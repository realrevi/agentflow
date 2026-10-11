import { describe, expect, it } from 'vitest';
import { MAX_LAG_MS, RevealBuffer } from './reveal';

/** Deterministic pseudo-random numbers (no Math.random in tests). */
function rng(seed: number) {
  let state = seed;
  return () => {
    state = (state * 1103515245 + 12345) % 2 ** 31;
    return state / 2 ** 31;
  };
}

/** Runs a stream through the buffer at 60 fps; returns each character's lag and per-frame counts. */
function simulate(deltas: { at: number; text: string }[], frameMs = 1000 / 60) {
  const buffer = new RevealBuffer();
  const arrivedAt: number[] = [];
  const shownAt: number[] = [];
  const perFrame: number[] = [];
  let next = 0;
  let shown = 0;
  const end = (deltas.at(-1)?.at ?? 0) + 2000;
  for (let now = 0; now <= end; now += frameMs) {
    while (next < deltas.length && (deltas[next]?.at ?? Infinity) <= now) {
      const delta = deltas[next++];
      if (!delta) break;
      for (let i = 0; i < delta.text.length; i++) arrivedAt.push(delta.at);
      buffer.push(delta.text, delta.at);
    }
    const visible = buffer.step(now).length;
    perFrame.push(visible - shown);
    for (let i = shown; i < visible; i++) shownAt[i] = now;
    shown = visible;
  }
  const lags = arrivedAt.map((at, i) => (shownAt[i] ?? Infinity) - at);
  return { lags, perFrame, buffer };
}

describe('reveal rate', () => {
  it('never trails the stream by more than the bound, even with huge bursts', () => {
    const random = rng(42);
    const deltas: { at: number; text: string }[] = [];
    let at = 0;
    for (let i = 0; i < 400; i++) {
      at += random() < 0.1 ? 400 * random() : 30 * random(); // pauses and bursts
      const size = random() < 0.05 ? 2000 : Math.ceil(random() * 12);
      deltas.push({ at, text: 'x'.repeat(size) });
    }
    const { lags, buffer } = simulate(deltas);
    expect(Math.max(...lags)).toBeLessThanOrEqual(MAX_LAG_MS);
    expect(buffer.backlog).toBe(0);
  });

  it('is smooth when tokens trickle in', () => {
    // ~40 chars per second in 4-char tokens: no frame dumps a whole token at once.
    const deltas = Array.from({ length: 50 }, (_, i) => ({ at: i * 100, text: 'abcd' }));
    const { perFrame, lags } = simulate(deltas);
    expect(Math.max(...perFrame)).toBeLessThanOrEqual(2);
    expect(Math.max(...lags)).toBeLessThanOrEqual(MAX_LAG_MS);
  });

  it('speeds up with the backlog', () => {
    const { perFrame } = simulate([{ at: 0, text: 'y'.repeat(3000) }]);
    const firstFrames = perFrame.slice(0, 3).reduce((a, b) => a + b, 0);
    expect(firstFrames).toBeGreaterThan(300);
  });

  it('flushes everything and never splits an emoji', () => {
    const buffer = new RevealBuffer();
    buffer.push('a🤖b', 0);
    const partial = buffer.step(16);
    expect(partial === '' || partial.length !== 2).toBe(true);
    for (let now = 16; buffer.backlog > 0; now += 16) {
      const text = buffer.step(now);
      expect(text.endsWith('\ud83e')).toBe(false);
    }
    expect(buffer.flush()).toBe('a🤖b');
  });
});
