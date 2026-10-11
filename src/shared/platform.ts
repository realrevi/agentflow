/** macOS uses Cmd where other platforms use Ctrl. */
export function isMac(): boolean {
  return typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.userAgent);
}

/** Label for a Ctrl/Cmd shortcut, e.g. `shortcutLabel('P')` -> "Ctrl+P" or "⌘P". */
export function shortcutLabel(key: string, mac = isMac()): string {
  return mac ? `⌘${key}` : `Ctrl+${key}`;
}

export function isWindows(): boolean {
  return typeof navigator !== 'undefined' && /Windows/.test(navigator.userAgent);
}
