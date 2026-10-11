// How each sprite state plays: loops while active, one-shots, or a held frame. Idle and
// sleep never animate, so nothing runs while agents rest.
import type { SpriteState } from './types';

export type Playback =
  | { kind: 'loop' }
  | { kind: 'once'; cycles: number; hold: 'first' | 'last' }
  | { kind: 'still'; frame: 'first' | 'last' };

const PLAYBACK: Record<SpriteState, Playback> = {
  idle: { kind: 'still', frame: 'first' },
  walk: { kind: 'loop' },
  think: { kind: 'loop' },
  work: { kind: 'loop' },
  receive: { kind: 'once', cycles: 1, hold: 'last' },
  done: { kind: 'loop' },
  error: { kind: 'once', cycles: 3, hold: 'first' },
  sleep: { kind: 'still', frame: 'last' },
};

/** `showcase` (the gallery) loops the resting states too. */
export function playbackOf(state: SpriteState, animate: boolean, showcase = false): Playback {
  const playback = PLAYBACK[state];
  if (animate && showcase && playback.kind === 'still') return { kind: 'loop' };
  if (animate || playback.kind === 'still') return playback;
  return { kind: 'still', frame: playback.kind === 'once' ? playback.hold : 'first' };
}

/** Frame to show `elapsed` ms after the state started; `null` once a one-shot ended. */
export function frameAt(
  playback: Playback,
  frames: number,
  elapsed: number,
  frameMs: number,
): number | null {
  if (playback.kind === 'still') return playback.frame === 'first' ? 0 : frames - 1;
  const step = Math.floor(Math.max(0, elapsed) / frameMs);
  if (playback.kind === 'loop') return step % frames;
  if (step >= frames * playback.cycles) return null;
  return step % frames;
}
