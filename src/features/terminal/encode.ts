// What the prompt bar writes to the pty for one prompt.
import { sanitizePaste } from './paste';

const PASTE_START = '\x1b[200~';
const PASTE_END = '\x1b[201~';

/**
 * Line breaks become CR like a terminal paste. Multiline text is wrapped in bracketed paste
 * when the program enabled it, so it arrives as one input instead of several Enters; then a
 * final CR sends it. Paste markers inside the text are removed so it cannot break out.
 */
export function encodePrompt(text: string, bracketedPaste: boolean): string {
  const body = sanitizePaste(text).replace(/\r\n|\r|\n/g, '\r');
  const multiline = body.includes('\r');
  return multiline && bracketedPaste ? `${PASTE_START}${body}${PASTE_END}\r` : `${body}\r`;
}
