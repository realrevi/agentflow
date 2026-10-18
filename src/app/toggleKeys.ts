// The pure parts of the side-area toggles (panelToggles.ts): which shortcut toggles what,
// and whether a click on a view leaves the sidebar open.
import { isMac } from '../shared/platform';
import type { SidebarView } from '../shared/types';

/** Clicking the view that is already shown hides the sidebar; anything else shows it. */
export function sidebarAfterSelect(
  ui: { sidebarVisible: boolean; activeView: SidebarView },
  view: SidebarView,
): boolean {
  return !(ui.sidebarVisible && ui.activeView === view);
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
