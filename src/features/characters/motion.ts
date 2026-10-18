// Smooth movement for characters and packets on the shared loop: frame-rate independent
// easing towards a target, an ease curve for flights, and snapping to whole sprite pixels
// at render so the pixel art stays crisp. Pure, except Glide's small map of positions.

export interface Point {
  x: number;
  y: number;
}

/** 0..1 -> 0..1, slow at both ends. */
export function easeInOutCubic(t: number): number {
  const c = Math.min(1, Math.max(0, t));
  return c < 0.5 ? 4 * c * c * c : 1 - (-2 * c + 2) ** 3 / 2;
}

/**
 * One step from `from` towards `to` after `dtMs`: covers half the remaining distance every
 * `halfLifeMs`, whatever the frame rate. Snaps once within half a pixel.
 */
export function approach(from: number, to: number, dtMs: number, halfLifeMs = 70): number {
  const k = 1 - 0.5 ** (Math.max(0, dtMs) / halfLifeMs);
  const next = from + (to - from) * k;
  return Math.abs(to - next) < 0.5 ? to : next;
}

/** `value` on the grid of whole sprite pixels (`pixel` CSS px each). */
export function snapToPixel(value: number, pixel: number): number {
  return pixel > 0 ? Math.round(value / pixel) * pixel : Math.round(value);
}

/** Where each character is shown while it glides to where it should be. */
export class Glide {
  private shown = new Map<string, Point>();
  private last = new Map<string, number>();
  private moving = new Set<string>();

  /** The shown position of `id` at `now`, moved towards `target` (jumps when not animating). */
  at(id: string, target: Point, now: number, animate: boolean, halfLifeMs = 70): Point {
    const current = this.shown.get(id);
    const previous = this.last.get(id) ?? now;
    this.last.set(id, now);
    if (!current || !animate) {
      this.shown.set(id, target);
      this.moving.delete(id);
      return target;
    }
    const dt = Math.min(64, now - previous);
    const next = {
      x: approach(current.x, target.x, dt, halfLifeMs),
      y: approach(current.y, target.y, dt, halfLifeMs),
    };
    this.shown.set(id, next);
    if (next.x === target.x && next.y === target.y) this.moving.delete(id);
    else this.moving.add(id);
    return next;
  }

  /** Characters still on their way (the loop runs while this is above 0). */
  get gliding(): number {
    return this.moving.size;
  }

  forget(id: string): void {
    this.shown.delete(id);
    this.last.delete(id);
    this.moving.delete(id);
  }
}
