import { terminalShortcut } from './keys';

/**
 * Ctrl/Cmd+Shift+` opens a new shell from anywhere. Registered in the capture phase so a
 * focused terminal never also receives the keystroke. Returns a cleanup function.
 */
export function installNewShellShortcut(openShell: () => void): () => void {
  const onKeyDown = (event: KeyboardEvent) => {
    if (terminalShortcut(event) !== 'newShell') return;
    event.preventDefault();
    event.stopPropagation();
    if (!event.repeat) openShell();
  };
  window.addEventListener('keydown', onKeyDown, true);
  return () => window.removeEventListener('keydown', onKeyDown, true);
}
