// A character on a canvas. Static states draw once; animated states join the shared
// loop only while the canvas is on screen.
import { createEffect, createSignal, on, onCleanup, onMount } from 'solid-js';
import { startAnimation, reducedMotion } from './animator';
import { characterById, characters, loadCharacters } from './catalog';
import { frameAt, playbackOf } from './playback';
import { deviceScale, drawFrame, strip } from './renderer';
import type { CharacterRef, SpriteState } from './types';

/** Length of the state-change squash and hop (characters.css). */
const HOP_MS = 260;

export interface AvatarProps {
  character: CharacterRef;
  state: SpriteState;
  /** Integer CSS scale (1 = 16 px for a 16x16 sheet). */
  scale: number;
  animate?: boolean;
  /** Animation speed multiplier (1 = the sheet's frameMs). */
  speed?: number;
  /** Gallery mode: idle and sleep loop instead of holding a frame. */
  showcase?: boolean;
  class?: string;
  title?: string;
  /** Called when a one-shot state (receive, error) has finished playing. */
  onFinished?: () => void;
}

export function Avatar(props: AvatarProps) {
  let canvas: HTMLCanvasElement | undefined;
  let visible = true;
  let stop: (() => void) | undefined;

  onMount(() => {
    void loadCharacters();
    if (!canvas || typeof IntersectionObserver === 'undefined') return;
    const observer = new IntersectionObserver((entries) => {
      visible = entries.some((entry) => entry.isIntersecting);
      if (visible) play();
    });
    observer.observe(canvas);
    onCleanup(() => observer.disconnect());
  });
  onCleanup(() => stop?.());

  const sheet = () => {
    characters(); // re-run once the catalog has loaded
    return characterById(props.character.id);
  };

  function play(): void {
    stop?.();
    stop = undefined;
    const current = sheet();
    const ctx = canvas?.getContext('2d');
    if (!canvas || !ctx || !current) return;
    const scale = deviceScale(props.scale);
    const size = current.size * scale;
    if (canvas.width !== size) canvas.width = canvas.height = size;
    const entry = strip(current, props.character.palette, scale, props.state);
    const animate = (props.animate ?? true) && !reducedMotion();
    const playback = playbackOf(props.state, animate, props.showcase);
    const frameMs = current.frameMs / Math.max(0.25, props.speed ?? 1);
    const startedAt = performance.now();
    const onFinished = props.onFinished;
    let shown = -1;
    const show = (frame: number) => {
      if (frame !== shown) drawFrame(ctx, entry, frame);
      shown = frame;
    };
    show(frameAt(playback, entry.frames, 0, frameMs) ?? 0);
    if (playback.kind === 'still' || !visible) {
      // A one-shot that cannot play (reduced motion, hidden) still ends right away.
      if (playbackOf(props.state, true).kind === 'once') queueMicrotask(() => onFinished?.());
      return;
    }
    stop = startAnimation({
      step(now) {
        if (!canvas?.isConnected || !visible) return false;
        const frame = frameAt(playback, entry.frames, now - startedAt, frameMs);
        if (frame === null) {
          show(playback.kind === 'once' && playback.hold === 'last' ? entry.frames - 1 : 0);
          onFinished?.();
          return false;
        }
        show(frame);
        return true;
      },
    });
  }

  createEffect(play);

  // A short squash-and-hop when the state changes, and a gentle breathing bob at rest: CSS
  // transforms in whole sprite pixels, so the shared loop stays stopped while idle.
  const moving = () => (props.animate ?? true) && !reducedMotion();
  const [hop, setHop] = createSignal(false);
  let hopTimer: ReturnType<typeof setTimeout> | undefined;
  createEffect(
    on(
      () => props.state,
      () => {
        if (!moving()) return;
        setHop(false);
        clearTimeout(hopTimer);
        queueMicrotask(() => setHop(true));
        hopTimer = setTimeout(() => setHop(false), HOP_MS);
      },
      { defer: true },
    ),
  );
  onCleanup(() => clearTimeout(hopTimer));

  return (
    <canvas
      ref={canvas}
      class={`af-avatar ${props.class ?? ''}`}
      classList={{
        'af-avatar-breathe': props.state === 'idle' && moving() && !hop(),
        'af-avatar-hop': hop(),
      }}
      role="img"
      aria-label={props.title ?? sheet()?.name ?? ''}
      title={props.title}
      style={{
        width: `${(sheet()?.size ?? 16) * props.scale}px`,
        height: `${(sheet()?.size ?? 16) * props.scale}px`,
        '--af-px': `${props.scale}px`,
      }}
    />
  );
}
