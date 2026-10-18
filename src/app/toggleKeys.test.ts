import { describe, expect, it } from 'vitest';
import { sidebarAfterSelect, toggleShortcut } from './toggleKeys';

function key(init: KeyboardEventInit): KeyboardEvent {
  return {
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    altKey: false,
    ...init,
  } as KeyboardEvent;
}

describe('side-area toggles', () => {
  it('maps Ctrl+B, Ctrl+J and Ctrl+Shift+E (Cmd on macOS)', () => {
    expect(toggleShortcut(key({ key: 'b', ctrlKey: true }), false)).toBe('sidebar');
    expect(toggleShortcut(key({ key: 'B', ctrlKey: true }), false)).toBe('sidebar');
    expect(toggleShortcut(key({ key: 'j', ctrlKey: true }), false)).toBe('terminal');
    expect(toggleShortcut(key({ key: 'E', ctrlKey: true, shiftKey: true }), false)).toBe('files');
    expect(toggleShortcut(key({ key: 'b', metaKey: true }), true)).toBe('sidebar');
  });

  it('ignores other modifiers and keys', () => {
    expect(toggleShortcut(key({ key: 'b' }), false)).toBeNull();
    expect(toggleShortcut(key({ key: 'b', metaKey: true }), false)).toBeNull();
    expect(toggleShortcut(key({ key: 'b', ctrlKey: true }), true)).toBeNull();
    expect(toggleShortcut(key({ key: 'b', ctrlKey: true, altKey: true }), false)).toBeNull();
    expect(toggleShortcut(key({ key: 'b', ctrlKey: true, shiftKey: true }), false)).toBeNull();
    expect(toggleShortcut(key({ key: 'e', ctrlKey: true }), false)).toBeNull();
  });

  it('clicking the active view hides the sidebar, any other shows it', () => {
    expect(sidebarAfterSelect({ sidebarVisible: true, activeView: 'agents' }, 'agents')).toBe(
      false,
    );
    expect(sidebarAfterSelect({ sidebarVisible: true, activeView: 'agents' }, 'usage')).toBe(true);
    expect(sidebarAfterSelect({ sidebarVisible: false, activeView: 'agents' }, 'agents')).toBe(
      true,
    );
  });
});
