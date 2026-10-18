// Pure Stage helpers: the packet's arc, hit tests, the bubble text from an activity, and
// whether anything needs animation frames at all (nothing animates when the Stage is idle).
import type { Activity } from '../agentui/activity';

export interface Point {
  x: number;
  y: number;
}

/** Point `t` (0..1) on an arc from `from` to `to` that rises above both. */
export function packetPoint(from: Point, to: Point, t: number): Point {
  const lift = Math.max(40, Math.hypot(to.x - from.x, to.y - from.y) * 0.3);
  const control = { x: (from.x + to.x) / 2, y: Math.min(from.y, to.y) - lift };
  const u = 1 - t;
  return {
    x: u * u * from.x + 2 * u * t * control.x + t * t * to.x,
    y: u * u * from.y + 2 * u * t * control.y + t * t * to.y,
  };
}

export function distanceToSegment(p: Point, a: Point, b: Point): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const length = dx * dx + dy * dy;
  const t =
    length === 0 ? 0 : Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / length));
  return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
}

export const BUBBLE_MAX = 40;
/** "Done" and errors fade this long after they happened; activity shows while it lasts. */
export const FADE_MS = 6000;

export interface BubbleView {
  text: string;
  full: string;
  visible: boolean;
  /** When it fades (ms, Date.now() clock), if it will. */
  fadesAt: number | null;
}

/** ~40 characters with an ellipsis (the full text goes in the tooltip). */
export function shortText(text: string, max = BUBBLE_MAX): string {
  const single = text.replace(/\s+/g, ' ').trim();
  return single.length > max ? `${single.slice(0, max - 1)}…` : single;
}

export function bubbleView(activity: Activity | null, now: number): BubbleView {
  const hidden = { text: '', full: '', visible: false, fadesAt: null };
  if (!activity || activity.kind === 'idle' || !activity.text.trim()) return hidden;
  const fades = activity.kind === 'done' || activity.kind === 'error';
  const fadesAt = fades ? activity.at + FADE_MS : null;
  if (fadesAt !== null && now >= fadesAt) return hidden;
  return { text: shortText(activity.text), full: activity.text, visible: true, fadesAt };
}

export interface MotionState {
  /** Stage animations on and no reduced motion. */
  animate: boolean;
  /** Characters whose sprite state animates. */
  animatedCharacters: number;
  packets: number;
  pulses: number;
  /** Pending semi-auto handoffs (a packet bounces at the sender). */
  pending: number;
  glows: number;
}

/** Whether the Stage needs the animation loop; a still Stage requests no frames. */
export function needsFrames(state: MotionState): boolean {
  if (!state.animate) return false;
  return (
    state.animatedCharacters > 0 ||
    state.packets > 0 ||
    state.pulses > 0 ||
    state.pending > 0 ||
    state.glows > 0
  );
}
