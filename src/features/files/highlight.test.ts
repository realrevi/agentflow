import { describe, expect, it } from 'vitest';
import { highlight } from './highlight';
import { isQuickOpenKey } from './quickOpenStore';

describe('quick open', () => {
  it('splits name and folder and marks matches', () => {
    const { name, folder } = highlight('src/main.rs', [0, 4, 5]);
    expect(name).toEqual([
      { text: 'ma', match: true },
      { text: 'in.rs', match: false },
    ]);
    expect(folder).toEqual([
      { text: 's', match: true },
      { text: 'rc', match: false },
    ]);
    expect(highlight('README.md', []).folder).toEqual([]);
  });

  it('counts code points like the backend', () => {
    const { name } = highlight('😀/é.md', [2]);
    expect(name[0]).toEqual({ text: 'é', match: true });
  });

  it('uses Cmd+P on macOS and Ctrl+P elsewhere', () => {
    const event = (init: KeyboardEventInit) =>
      ({
        key: 'p',
        ctrlKey: false,
        metaKey: false,
        shiftKey: false,
        altKey: false,
        ...init,
      }) as KeyboardEvent;
    expect(isQuickOpenKey(event({ ctrlKey: true, metaKey: false }), false)).toBe(true);
    expect(isQuickOpenKey(event({ metaKey: true, ctrlKey: false }), true)).toBe(true);
    expect(isQuickOpenKey(event({ ctrlKey: true, shiftKey: true }), false)).toBe(false);
    expect(isQuickOpenKey(event({ ctrlKey: true }), true)).toBe(false);
  });
});
