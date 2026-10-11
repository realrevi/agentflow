// The character list, loaded once on first use (sprite data is its own chunk).
import { createSignal } from 'solid-js';
import type { CharacterRef, SpriteSheet } from './types';

const [sheets, setSheets] = createSignal<readonly SpriteSheet[]>([]);
let loading: Promise<readonly SpriteSheet[]> | undefined;

/** Reactive; empty until `loadCharacters()` resolves. */
export const characters = sheets;

export function loadCharacters(): Promise<readonly SpriteSheet[]> {
  loading ??= import('./spriteData').then(({ SPRITES }) => {
    setSheets(SPRITES);
    return SPRITES;
  });
  return loading;
}

export function characterById(id: string): SpriteSheet | undefined {
  return sheets().find((sheet) => sheet.id === id);
}

/** A random character nobody uses yet (any of them once all are taken). */
export function pickCharacter(
  all: readonly SpriteSheet[],
  used: readonly string[],
  random: () => number = Math.random,
): CharacterRef | null {
  if (all.length === 0) return null;
  const free = all.filter((sheet) => !used.includes(sheet.id));
  const pool = free.length > 0 ? free : all;
  const sheet = pool[Math.floor(random() * pool.length)] ?? pool[0];
  return sheet ? { id: sheet.id, palette: 0 } : null;
}

export function isCharacterRef(value: unknown): value is CharacterRef {
  if (typeof value !== 'object' || value === null) return false;
  const { id, palette } = value as Record<string, unknown>;
  return (
    typeof id === 'string' &&
    id.length > 0 &&
    typeof palette === 'number' &&
    Number.isInteger(palette) &&
    palette >= 0
  );
}
