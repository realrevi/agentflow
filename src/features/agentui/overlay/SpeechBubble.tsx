// A small bubble with a typewriter reveal (~50 chars/s) on the shared animation loop.
import { createEffect, createSignal, onCleanup } from 'solid-js';
import { reducedMotion, startAnimation } from '../../characters';

const CHARS_PER_SECOND = 50;

export function SpeechBubble(props: { text: string; typewriter: boolean; speed: number }) {
  const [shown, setShown] = createSignal('');
  let stop: (() => void) | undefined;

  createEffect(() => {
    const text = props.text;
    stop?.();
    stop = undefined;
    if (!props.typewriter || reducedMotion()) {
      setShown(text);
      return;
    }
    const startedAt = performance.now();
    const rate = (CHARS_PER_SECOND * Math.max(0.25, props.speed)) / 1000;
    setShown('');
    stop = startAnimation({
      step(now) {
        const count = Math.min(text.length, Math.floor((now - startedAt) * rate) + 1);
        setShown(text.slice(0, count));
        return count < text.length;
      },
    });
  });
  onCleanup(() => stop?.());

  return (
    <div class="af-bubble" role="status" aria-label={props.text}>
      {shown()}
    </div>
  );
}
