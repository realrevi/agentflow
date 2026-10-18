import { describe, expect, it } from 'vitest';
import { approach, easeInOutCubic, Glide, snapToPixel } from './motion';

describe('character motion', () => {
  it('eases in and out between 0 and 1', () => {
    expect(easeInOutCubic(0)).toBe(0);
    expect(easeInOutCubic(1)).toBe(1);
    expect(easeInOutCubic(0.5)).toBeCloseTo(0.5);
    expect(easeInOutCubic(0.1)).toBeLessThan(0.1);
    expect(easeInOutCubic(2)).toBe(1);
  });

  it('approaches at the same pace whatever the frame rate', () => {
    let slow = 0;
    let fast = 0;
    for (let i = 0; i < 2; i++) slow = approach(slow, 100, 35, 70);
    for (let i = 0; i < 7; i++) fast = approach(fast, 100, 10, 70);
    expect(slow).toBeCloseTo(fast, 0);
    expect(approach(99.8, 100, 16)).toBe(100);
  });

  it('snaps to whole sprite pixels', () => {
    expect(snapToPixel(13, 3)).toBe(12);
    expect(snapToPixel(14, 3)).toBe(15);
    expect(snapToPixel(7.4, 0)).toBe(7);
  });

  it('glides towards the target and reports when it has arrived', () => {
    const glide = new Glide();
    expect(glide.at('a', { x: 0, y: 0 }, 0, true)).toEqual({ x: 0, y: 0 });
    const mid = glide.at('a', { x: 100, y: 0 }, 16, true);
    expect(mid.x).toBeGreaterThan(0);
    expect(mid.x).toBeLessThan(100);
    expect(glide.gliding).toBe(1);
    let at = 16;
    let shown = mid;
    while (glide.gliding > 0 && at < 5000) {
      at += 16;
      shown = glide.at('a', { x: 100, y: 0 }, at, true);
    }
    expect(shown).toEqual({ x: 100, y: 0 });
    expect(glide.gliding).toBe(0);
  });

  it('jumps without animation (reduced motion)', () => {
    const glide = new Glide();
    glide.at('a', { x: 0, y: 0 }, 0, true);
    expect(glide.at('a', { x: 50, y: 20 }, 16, false)).toEqual({ x: 50, y: 20 });
    expect(glide.gliding).toBe(0);
  });
});
