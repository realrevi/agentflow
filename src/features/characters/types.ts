// Pixel characters: sprite sheets are JSON data (sprites/*.json), never images.

export const SPRITE_STATES = [
  'idle',
  'walk',
  'think',
  'work',
  'receive',
  'done',
  'error',
  'sleep',
] as const;
export type SpriteState = (typeof SPRITE_STATES)[number];

/** One frame: `size` strings of `size` chars; '.' is transparent, other chars index the palette. */
export type SpriteFrame = readonly string[];

export interface SpriteSheet {
  id: string;
  /** Same in every language. */
  name: string;
  /** 16 today; 32x32 sheets work too. */
  size: number;
  /** Color variants; every palette has the same number of hex colors. */
  palettes: readonly (readonly string[])[];
  frameMs: number;
  states: Readonly<Record<SpriteState, readonly SpriteFrame[]>>;
}

/** Which character (and color variant) a panel shows. Persisted with the panel config. */
export interface CharacterRef {
  id: string;
  palette: number;
}
