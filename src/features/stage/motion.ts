// What moves on the Stage besides the characters' own sprites: packets travelling from the
// sender to the receiver (the receiver then plays "receive"), the brightened line and the
// aura of a handoff, and a pending handoff's packet bouncing at its sender. Plain data
// (read while drawing), plus the one loop on the shared animator, run only while needed.
import { startAnimation } from '../characters/animator';
import { easeInOutCubic } from '../characters/motion';
import { packetPoint, type Point } from './scene';

export const FLIGHT_MS = 900;
export const PULSE_MS = 1600;
const BOUNCE_MS = 150;

interface Flight {
  from: string;
  to: string;
  color: string;
  start: number;
}

export function pairKey(a: string, b: string): string {
  return a < b ? `${a}|${b}` : `${b}|${a}`;
}

export class StageMotion {
  private flights: Flight[] = [];
  /** Pair -> until (performance.now()). */
  private pulses = new Map<string, { until: number; color: string }>();
  /** Panels playing "receive", since when. */
  readonly receiving = new Map<string, number>();
  private stop: (() => void) | undefined;

  /**
   * A handoff went out. `travel`: the packet flies (else only the line lights up, e.g. with
   * reduced motion).
   */
  handoff(from: string, to: string, color: string, now: number, travel: boolean): void {
    this.pulses.set(pairKey(from, to), { until: now + PULSE_MS, color });
    if (travel && from !== to) this.flights.push({ from, to, color, start: now });
    else this.receiving.set(to, now);
  }

  /** Moves time on: landed packets start the receiver's "receive". */
  advance(now: number): void {
    const landed = this.flights.filter((flight) => now - flight.start >= FLIGHT_MS);
    this.flights = this.flights.filter((flight) => now - flight.start < FLIGHT_MS);
    for (const flight of landed) this.receiving.set(flight.to, now);
    for (const [key, pulse] of this.pulses) if (now >= pulse.until) this.pulses.delete(key);
  }

  /** 0..1: how hot the line between `a` and `b` is. */
  heat(a: string, b: string, now: number): number {
    const pulse = this.pulses.get(pairKey(a, b));
    return pulse ? Math.max(0, Math.min(1, (pulse.until - now) / PULSE_MS)) : 0;
  }

  /** The color of a running handoff around `panel`, if any. */
  aura(panel: string, now: number): string | null {
    for (const [key, pulse] of this.pulses) {
      if (now < pulse.until && key.split('|').includes(panel)) return pulse.color;
    }
    return null;
  }

  packets(pointOf: (panel: string) => Point | undefined, now: number) {
    return this.flights.flatMap((flight) => {
      const from = pointOf(flight.from);
      const to = pointOf(flight.to);
      if (!from || !to) return [];
      const t = Math.min(1, (now - flight.start) / FLIGHT_MS);
      return [{ at: packetPoint(from, to, easeInOutCubic(t)), color: flight.color }];
    });
  }

  /** A packet bobbing above a sender whose handoff waits for a click. */
  static bounce(at: Point, lift: number, now: number, animate: boolean): Point {
    const hop = animate ? Math.abs(Math.sin(now / BOUNCE_MS)) * 8 : 0;
    return { x: at.x, y: at.y - lift - hop };
  }

  get busy(): number {
    return this.flights.length + this.pulses.size;
  }

  /** Runs `frame` on the shared loop until it returns false. One loop at most. */
  run(frame: (now: number) => boolean): void {
    if (this.stop) return;
    this.stop = startAnimation({
      step: (now) => {
        const keep = frame(now);
        if (!keep) this.stop = undefined;
        return keep;
      },
    });
  }

  halt(): void {
    this.stop?.();
    this.stop = undefined;
  }

  get running(): boolean {
    return this.stop !== undefined;
  }
}
