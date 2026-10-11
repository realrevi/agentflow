import { describe, expect, it } from 'vitest';
import { isHexColor, parseHex, readableOn, toHex6, withAlpha } from './color';

describe('color helpers', () => {
  it('validates the VS Code hex formats', () => {
    for (const ok of ['#abc', '#abcd', '#A1B2C3', '#a1b2c3d4']) expect(isHexColor(ok)).toBe(true);
    for (const bad of ['abc', '#ab', '#abcde', 'red', '#gggggg'])
      expect(isHexColor(bad)).toBe(false);
  });

  it('parses short and alpha forms', () => {
    expect(parseHex('#f80')).toEqual({ r: 255, g: 136, b: 0, a: 1 });
    expect(parseHex('#00000080')?.a).toBeCloseTo(0.5, 2);
    expect(parseHex('nope')).toBeNull();
  });

  it('converts for color inputs and rgba', () => {
    expect(toHex6('#F5C542cc')).toBe('#f5c542');
    expect(toHex6('#abc')).toBe('#aabbcc');
    expect(withAlpha('#ffffff', 0.5)).toBe('rgba(255, 255, 255, 0.5)');
  });

  it('picks a readable text color', () => {
    expect(readableOn('#ffffff')).toBe('#000000');
    expect(readableOn('#16171d')).toBe('#ffffff');
  });
});
