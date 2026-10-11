// Small helpers for TerminalController: fonts, exit messages, mode resets.
import type { PtyExitMessage } from '../../shared/types';

export const FONT_FAMILY = "'JetBrains Mono', ui-monospace, Menlo, Consolas, monospace";

let fonts: Promise<unknown> | undefined;
/** xterm measures glyphs on open, so the bundled font must be loaded first. */
export function fontsReady(): Promise<unknown> {
  fonts ??= document.fonts.load('13px "JetBrains Mono"').catch(() => undefined);
  return fonts;
}

export function isExit(message: unknown): message is PtyExitMessage {
  return (
    typeof message === 'object' && message !== null && 'type' in message && message.type === 'exit'
  );
}

/**
 * After a process exits, undo modes it may have left on (it could have crashed): leave the
 * alternate screen, disable mouse tracking and bracketed paste, show the cursor, normal cursor
 * keys, reset colors.
 */
export const RESET_MODES =
  '\x1b[?1049l\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1015l' +
  '\x1b[?2004l\x1b[?25h\x1b[?1l\x1b>\x1b[0m';

/** Where the cursor is, in cells of the visible screen (`row` < 0 or >= rows: scrolled away). */
export interface CursorCell {
  col: number;
  row: number;
  cols: number;
  rows: number;
}
