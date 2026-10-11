// Pixels -> bitmaps. Each (character, palette, scale, state) strip is drawn once into an
// offscreen canvas, turned into an ImageBitmap and cached (LRU), so avatars only copy
// frames with drawImage and image smoothing off.
import type { SpriteSheet, SpriteState } from './types';

export interface Strip {
  /** The canvas until its ImageBitmap is ready, then the bitmap. */
  source: CanvasImageSource;
  cell: number;
  frames: number;
}

const MAX_STRIPS = 96;
const cache = new Map<string, Strip>();

function makeCanvas(width: number, height: number): HTMLCanvasElement | OffscreenCanvas {
  if (typeof OffscreenCanvas !== 'undefined') return new OffscreenCanvas(width, height);
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  return canvas;
}

/** Device pixels per sprite pixel: the integer CSS scale times the (rounded) pixel ratio. */
export function deviceScale(scale: number): number {
  const ratio =
    typeof window === 'undefined' ? 1 : Math.max(1, Math.round(window.devicePixelRatio));
  return Math.max(1, Math.round(scale)) * ratio;
}

export function paletteOf(sheet: SpriteSheet, palette: number): readonly string[] {
  return sheet.palettes[palette] ?? sheet.palettes[0] ?? [];
}

export function strip(
  sheet: SpriteSheet,
  palette: number,
  scale: number,
  state: SpriteState,
): Strip {
  const key = `${sheet.id}:${palette}:${scale}:${state}`;
  const hit = cache.get(key);
  if (hit) {
    cache.delete(key); // refresh its LRU position
    cache.set(key, hit);
    return hit;
  }
  const frames = sheet.states[state];
  const cell = sheet.size * scale;
  const canvas = makeCanvas(cell * frames.length, cell);
  const ctx = canvas.getContext('2d') as
    CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D | null;
  const colors = paletteOf(sheet, palette);
  frames.forEach((frame, index) => {
    frame.forEach((row, y) => {
      // Merge runs of one color into a single rect.
      let x = 0;
      while (x < row.length) {
        const char = row.charAt(x);
        let end = x + 1;
        while (row.charAt(end) === char) end++;
        const color = char === '.' ? undefined : colors[Number.parseInt(char, 36)];
        if (ctx && color) {
          ctx.fillStyle = color;
          ctx.fillRect(index * cell + x * scale, y * scale, (end - x) * scale, scale);
        }
        x = end;
      }
    });
  });
  const entry: Strip = { source: canvas, cell, frames: frames.length };
  if (typeof createImageBitmap === 'function') {
    void createImageBitmap(canvas).then(
      (bitmap) => (entry.source = bitmap),
      () => undefined,
    );
  }
  cache.set(key, entry);
  evict();
  return entry;
}

function evict(): void {
  while (cache.size > MAX_STRIPS) {
    const oldest = cache.keys().next().value;
    if (oldest === undefined) return;
    const entry = cache.get(oldest);
    if (entry && typeof ImageBitmap !== 'undefined' && entry.source instanceof ImageBitmap) {
      entry.source.close();
    }
    cache.delete(oldest);
  }
}

/** Copies one frame onto `ctx` at the origin (the canvas is the size of one cell). */
export function drawFrame(ctx: CanvasRenderingContext2D, entry: Strip, frame: number): void {
  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
  const index = Math.max(0, Math.min(entry.frames - 1, frame));
  ctx.drawImage(
    entry.source,
    index * entry.cell,
    0,
    entry.cell,
    entry.cell,
    0,
    0,
    entry.cell,
    entry.cell,
  );
}
