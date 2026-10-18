// The ONE requestAnimationFrame loop for every animated avatar, packet and typewriter.
// It runs only while something animates and the page is visible; otherwise no frame is
// requested at all, so idle CPU stays ~0.
import { createSignal } from 'solid-js';

export interface Animation {
  /** Called once per frame; return false when finished (it is then dropped). */
  step(now: number): boolean;
}

const running = new Set<Animation>();
let handle = 0;
let frames = 0;

function pageVisible(): boolean {
  return typeof document === 'undefined' || document.visibilityState !== 'hidden';
}

function schedule(): void {
  if (handle || running.size === 0 || !pageVisible()) return;
  handle = requestAnimationFrame(tick);
}

function tick(now: number): void {
  handle = 0;
  frames++;
  for (const animation of [...running]) {
    if (!animation.step(now)) running.delete(animation);
  }
  schedule();
}

/** Adds an animation to the shared loop. Returns a function that removes it. */
export function startAnimation(animation: Animation): () => void {
  running.add(animation);
  schedule();
  return () => {
    running.delete(animation);
  };
}

/** Diagnostics for tests: animations alive and frames run so far. */
export function animatorStats(): { running: number; frames: number; scheduled: boolean } {
  return { running: running.size, frames, scheduled: handle !== 0 };
}

if (typeof document !== 'undefined') {
  document.addEventListener('visibilitychange', () => {
    if (pageVisible()) schedule();
    else if (handle) {
      cancelAnimationFrame(handle);
      handle = 0;
    }
  });
}

// CSS idle motion (the avatars' breathing) pauses while the window is hidden or unfocused,
// like the loop above: a paused CSS animation asks for no frames.
if (typeof window !== 'undefined' && typeof document !== 'undefined') {
  const rest = () =>
    document.documentElement.classList.toggle('af-at-rest', !pageVisible() || !document.hasFocus());
  window.addEventListener('focus', rest);
  window.addEventListener('blur', rest);
  document.addEventListener('visibilitychange', rest);
  rest();
}

const query =
  typeof window !== 'undefined' && typeof window.matchMedia === 'function'
    ? window.matchMedia('(prefers-reduced-motion: reduce)')
    : undefined;
const [reducedMotion, setReducedMotion] = createSignal(query?.matches ?? false);
query?.addEventListener('change', (event) => setReducedMotion(event.matches));

/** True when the OS asks for less motion: avatars hold still and packets do not fly. */
export { reducedMotion };
