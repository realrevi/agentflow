// Checks a sprite sheet loaded from JSON, so a broken or hand-made sheet fails loudly.
import { SPRITE_STATES, type SpriteSheet } from './types';

const HEX = /^#[0-9a-fA-F]{6}$/;
export const SPRITE_SIZES: readonly number[] = [16, 32];
export const FRAMES_PER_STATE = { min: 2, max: 6 } as const;
export const PALETTES_PER_SHEET = { min: 2, max: 3 } as const;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function fail(id: string, message: string): never {
  throw new Error(`sprite "${id}": ${message}`);
}

/** Returns the sheet typed, or throws with the first problem found. */
export function validateSprite(raw: unknown): SpriteSheet {
  if (!isRecord(raw)) throw new Error('sprite: not an object');
  const { id, name, size, palettes, frameMs, states } = raw;
  if (typeof id !== 'string' || !/^[a-z][a-z0-9-]*$/.test(id)) throw new Error('sprite: bad id');
  if (typeof name !== 'string' || !name.trim()) fail(id, 'missing name');
  if (typeof size !== 'number' || !SPRITE_SIZES.includes(size)) fail(id, `size must be 16 or 32`);
  if (typeof frameMs !== 'number' || frameMs < 40 || frameMs > 2000) fail(id, 'bad frameMs');
  if (!Array.isArray(palettes)) fail(id, 'palettes must be an array');
  const count = palettes.length;
  if (count < PALETTES_PER_SHEET.min || count > PALETTES_PER_SHEET.max) {
    fail(id, `needs ${PALETTES_PER_SHEET.min}-${PALETTES_PER_SHEET.max} palettes`);
  }
  const colors = Array.isArray(palettes[0]) ? palettes[0].length : 0;
  for (const palette of palettes) {
    if (!Array.isArray(palette) || palette.length !== colors || colors === 0) {
      fail(id, 'palettes must all have the same number of colors');
    }
    if (!palette.every((color) => typeof color === 'string' && HEX.test(color))) {
      fail(id, 'palette colors must be #rrggbb');
    }
  }
  if (!isRecord(states)) fail(id, 'missing states');
  for (const state of SPRITE_STATES) {
    const frames = states[state];
    if (!Array.isArray(frames)) fail(id, `missing state "${state}"`);
    if (frames.length < FRAMES_PER_STATE.min || frames.length > FRAMES_PER_STATE.max) {
      fail(id, `"${state}" needs ${FRAMES_PER_STATE.min}-${FRAMES_PER_STATE.max} frames`);
    }
    frames.forEach((frame: unknown, index: number) =>
      checkFrame(id, `${state}[${index}]`, frame, size, colors),
    );
  }
  return raw as unknown as SpriteSheet;
}

function checkFrame(id: string, where: string, frame: unknown, size: number, colors: number): void {
  if (!Array.isArray(frame) || frame.length !== size) fail(id, `${where} must have ${size} rows`);
  frame.forEach((row: unknown, y: number) => {
    if (typeof row !== 'string' || row.length !== size) {
      fail(id, `${where} row ${y} must be ${size} chars`);
    }
    for (const char of row) {
      if (char === '.') continue;
      const index = Number.parseInt(char, 36);
      if (!(index >= 0 && index < colors)) fail(id, `${where} row ${y}: "${char}" is not a color`);
    }
  });
}
