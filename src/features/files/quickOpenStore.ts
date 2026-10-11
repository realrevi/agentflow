// Ctrl/Cmd+P toggles the quick open palette from anywhere (capture phase, so a focused
// terminal or editor does not also get the keystroke).
import { createSignal } from 'solid-js';
import { isMac } from '../../shared/platform';

const [quickOpenVisible, setQuickOpenVisible] = createSignal(false);

export { quickOpenVisible };

export function openQuickOpen(): void {
  setQuickOpenVisible(true);
}

export function closeQuickOpen(): void {
  setQuickOpenVisible(false);
}

export function isQuickOpenKey(event: KeyboardEvent, mac = isMac()): boolean {
  const modifier = mac ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
  return modifier && !event.shiftKey && !event.altKey && event.key.toLowerCase() === 'p';
}

/** Returns a cleanup function. */
export function installQuickOpenShortcut(): () => void {
  const onKeyDown = (event: KeyboardEvent) => {
    if (!isQuickOpenKey(event)) return;
    event.preventDefault();
    event.stopPropagation();
    if (!event.repeat) setQuickOpenVisible((visible) => !visible);
  };
  window.addEventListener('keydown', onKeyDown, true);
  return () => window.removeEventListener('keydown', onKeyDown, true);
}
