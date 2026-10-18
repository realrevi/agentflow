// Keeps an element mounted for its exit animation: `mounted` turns false only after
// `closing` has been true for `ms` (0 with reduced motion, so it closes at once).
import { createEffect, createSignal, on, onCleanup } from 'solid-js';

export function reducedMotion(): boolean {
  return (
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  );
}

export function createPresence(when: () => boolean, ms = 160) {
  const [mounted, setMounted] = createSignal(when());
  const [closing, setClosing] = createSignal(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  createEffect(
    on(when, (open) => {
      clearTimeout(timer);
      if (open) {
        setClosing(false);
        setMounted(true);
        return;
      }
      if (!mounted()) return;
      const delay = reducedMotion() ? 0 : ms;
      if (delay === 0) {
        setMounted(false);
        return;
      }
      setClosing(true);
      timer = setTimeout(() => {
        setClosing(false);
        setMounted(false);
      }, delay);
    }),
  );
  onCleanup(() => clearTimeout(timer));
  return { mounted, closing };
}
