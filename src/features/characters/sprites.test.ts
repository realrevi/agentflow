import { describe, expect, it } from 'vitest';
import { pickCharacter } from './catalog';
import { SPRITES } from './spriteData';
import { SPRITE_STATES } from './types';
import { validateSprite } from './validate';

describe('sprite sheets', () => {
  it('has exactly 15 characters with unique ids and names', () => {
    expect(SPRITES).toHaveLength(15);
    expect(new Set(SPRITES.map((s) => s.id)).size).toBe(15);
    expect(new Set(SPRITES.map((s) => s.name)).size).toBe(15);
  });

  it('every frame matches the size and every pixel maps to a palette color', () => {
    for (const sheet of SPRITES) {
      expect(sheet.size, sheet.id).toBe(16);
      for (const state of SPRITE_STATES) {
        const frames = sheet.states[state];
        expect(frames.length, `${sheet.id}.${state}`).toBeGreaterThanOrEqual(2);
        expect(frames.length, `${sheet.id}.${state}`).toBeLessThanOrEqual(4);
        for (const frame of frames) {
          expect(frame).toHaveLength(sheet.size);
          for (const row of frame) {
            expect(row).toHaveLength(sheet.size);
            for (const char of row) {
              if (char === '.') continue;
              const colors = sheet.palettes[0]?.length ?? 0;
              expect(Number.parseInt(char, 36), `${sheet.id} "${char}"`).toBeLessThan(colors);
            }
          }
        }
      }
    }
  });

  it('every sheet has all 8 states and 2-3 palettes of equal length', () => {
    for (const sheet of SPRITES) {
      expect(Object.keys(sheet.states).sort()).toEqual([...SPRITE_STATES].sort());
      expect(sheet.palettes.length).toBeGreaterThanOrEqual(2);
      expect(sheet.palettes.length).toBeLessThanOrEqual(3);
      expect(new Set(sheet.palettes.map((p) => p.length)).size).toBe(1);
    }
  });

  it('rejects broken sheets', () => {
    const first = SPRITES[0];
    if (!first) throw new Error('no sprites');
    const good = structuredClone(first);
    expect(() => validateSprite(good)).not.toThrow();
    const badRow = structuredClone(good) as unknown as { states: { idle: string[][] } };
    badRow.states.idle[0] = ['...'];
    expect(() => validateSprite(badRow)).toThrow(/rows/);
    const badColor = structuredClone(good) as unknown as { states: { walk: string[][] } };
    const frame = badColor.states.walk[0] ?? [];
    frame[0] = 'z'.repeat(16);
    expect(() => validateSprite(badColor)).toThrow(/not a color/);
    const { states, ...noStates } = good;
    expect(states).toBeDefined();
    expect(() => validateSprite(noStates)).toThrow(/states/);
  });

  it('accepts a 32x32 sheet', () => {
    const blank = Array.from({ length: 32 }, () => '.'.repeat(32));
    const states = Object.fromEntries(SPRITE_STATES.map((s) => [s, [blank, blank]]));
    const big = {
      id: 'big',
      name: 'Big',
      size: 32,
      frameMs: 200,
      palettes: [['#000000'], ['#ffffff']],
      states,
    };
    expect(validateSprite(big).size).toBe(32);
  });

  it('picks an unused character first', () => {
    const used = SPRITES.slice(0, 14).map((s) => s.id);
    expect(pickCharacter(SPRITES, used)?.id).toBe(SPRITES[14]?.id);
    expect(
      pickCharacter(
        SPRITES,
        SPRITES.map((s) => s.id),
        () => 0,
      )?.id,
    ).toBe(SPRITES[0]?.id);
    expect(pickCharacter([], [])).toBeNull();
  });
});
