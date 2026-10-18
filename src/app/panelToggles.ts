// Showing and hiding the side areas: the sidebar (the active activity icon, the edge
// handle, Ctrl/Cmd+B), the bottom terminal group (Ctrl/Cmd+J) and the Files panel
// (Ctrl/Cmd+Shift+E), like VS Code. Shortcuts run in the capture phase so a focused
// terminal or editor does not also get them.
import type { SidebarView } from '../shared/types';
import { isMac } from '../shared/platform';
import { togglePanelArea } from '../features/layout';
import { settings, updateSettings } from '../features/settings';

/** Clicking the active view hides the sidebar; any other view shows it. */
export function selectView(view: SidebarView): void {
  updateSettings((s) => {
    s.ui.sidebarVisible = !(s.ui.sidebarVisible && s.ui.activeView === view);
    s.ui.activeView = view;
  });
}

export function toggleSidebar(): void {
  updateSettings((s) => (s.ui.sidebarVisible = !settings.ui.sidebarVisible));
}

export type ToggleAction = 'sidebar' | 'terminal' | 'files';

/** The area a shortcut toggles, or null. */
export function toggleShortcut(event: KeyboardEvent, mac = isMac()): ToggleAction | null {
  const modifier = mac ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
  if (!modifier || event.altKey) return null;
  const key = event.key.toLowerCase();
  if (key === 'b' && !event.shiftKey) return 'sidebar';
  if (key === 'j' && !event.shiftKey) return 'terminal';
  if (key === 'e' && event.shiftKey) return 'files';
  return null;
}

export function runToggle(action: ToggleAction): void {
  if (action === 'sidebar') toggleSidebar();
  else togglePanelArea(action);
}

/** Returns a cleanup function. */
export function installToggleShortcuts(): () => void {
  const onKeyDown = (event: KeyboardEvent) => {
    const action = toggleShortcut(event);
    if (!action) return;
    event.preventDefault();
    event.stopPropagation();
    if (!event.repeat) runToggle(action);
  };
  window.addEventListener('keydown', onKeyDown, true);
  return () => window.removeEventListener('keydown', onKeyDown, true);
}
