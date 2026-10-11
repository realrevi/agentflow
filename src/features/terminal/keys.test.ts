import { describe, expect, it } from 'vitest';
import { terminalShortcut } from './keys';

const key = (init: Partial<KeyboardEvent> & { code: string; key: string }) => ({
  ctrlKey: false,
  metaKey: false,
  shiftKey: false,
  altKey: false,
  ...init,
});

describe('terminalShortcut', () => {
  it('leaves plain Ctrl+C to the process (SIGINT)', () => {
    expect(terminalShortcut(key({ ctrlKey: true, code: 'KeyC', key: 'c' }), false)).toBeNull();
  });

  it('maps Ctrl+Shift shortcuts on Windows/Linux', () => {
    const shift = { ctrlKey: true, shiftKey: true };
    expect(terminalShortcut(key({ ...shift, code: 'KeyC', key: 'C' }), false)).toBe('copy');
    expect(terminalShortcut(key({ ...shift, code: 'KeyV', key: 'V' }), false)).toBe('paste');
    expect(terminalShortcut(key({ ...shift, code: 'Backquote', key: '~' }), false)).toBe(
      'newShell',
    );
  });

  it('uses Cmd on macOS and ignores Ctrl there', () => {
    const cmd = { metaKey: true, shiftKey: true };
    expect(terminalShortcut(key({ ...cmd, code: 'KeyV', key: 'v' }), true)).toBe('paste');
    const ctrl = { ctrlKey: true, shiftKey: true };
    expect(terminalShortcut(key({ ...ctrl, code: 'KeyV', key: 'v' }), true)).toBeNull();
  });

  it('ignores combinations with Alt', () => {
    const combo = { ctrlKey: true, shiftKey: true, altKey: true };
    expect(terminalShortcut(key({ ...combo, code: 'KeyC', key: 'C' }), false)).toBeNull();
  });
});
