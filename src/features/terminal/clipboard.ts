// Copy/paste for the terminal shortcuts, through the native clipboard plugin.
import type { Terminal } from '@xterm/xterm';
import { ipc } from '../../shared/ipc';
import { reportError } from '../i18n';
import { sanitizePaste } from './paste';

export async function copySelection(term: Terminal): Promise<void> {
  const text = term.getSelection();
  if (!text) return;
  try {
    await ipc.writeClipboard(text);
  } catch (err) {
    reportError('error.clipboard', err);
  }
}

/** Pastes clipboard text (bracketed if the program asked for it). */
export async function pasteInto(term: Terminal, isDisposed: () => boolean): Promise<void> {
  try {
    const text = await ipc.readClipboard();
    if (text && !isDisposed()) term.paste(sanitizePaste(text));
  } catch (err) {
    reportError('error.clipboard', err);
  }
}
