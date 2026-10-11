// Pasted text is wrapped in bracketed-paste markers (ESC[200~ ... ESC[201~) so shells treat it
// as text. If the clipboard itself contains an end marker, the rest would run as typed
// commands, so markers (7-bit and 8-bit CSI forms) are removed first.
const MARKERS = /(?:\x1b\[|\x9b)20[01]~/g;

export function sanitizePaste(text: string): string {
  return text.replace(MARKERS, '');
}
