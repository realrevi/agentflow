// Terminal shortcuts. Ctrl+Shift (Cmd+Shift on macOS) so plain Ctrl+C still reaches the
// process as SIGINT.
import { isMac } from '../../shared/platform';

export type TerminalShortcut = 'copy' | 'paste' | 'newShell';

type KeyInfo = Pick<KeyboardEvent, 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey' | 'code' | 'key'>;

export function terminalShortcut(event: KeyInfo, mac = isMac()): TerminalShortcut | null {
  const modifier = mac ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
  if (!modifier || !event.shiftKey || event.altKey) return null;
  const key = event.key.toLowerCase();
  if (event.code === 'KeyC' || key === 'c') return 'copy';
  if (event.code === 'KeyV' || key === 'v') return 'paste';
  if (event.code === 'Backquote' || key === '`' || key === '~') return 'newShell';
  return null;
}
