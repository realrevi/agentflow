// Every sheet in sprites/, bundled into one lazily loaded chunk (see catalog.ts).
// Dropping a new JSON file in the folder is enough to add a character.
import type { SpriteSheet } from './types';
import { validateSprite } from './validate';

const files = import.meta.glob<unknown>('./sprites/*.json', { eager: true, import: 'default' });

/** Validated sheets in a stable order (by file name). */
export const SPRITES: readonly SpriteSheet[] = Object.keys(files)
  .sort()
  .map((path) => validateSprite(files[path]));
