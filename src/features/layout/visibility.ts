// Show or hide the bottom terminal group and the right Files group (status bar buttons,
// shortcuts). A hidden group comes back when a panel opens in it; with no group of that
// role yet, toggling opens its first panel instead.
import { createSignal } from 'solid-js';
import type { DockviewApi } from 'dockview-core';
import { groupRole, type GroupRole } from './roles';

export type ToggleRole = Extract<GroupRole, 'terminal' | 'files'>;

const [hiddenRoles, setHiddenRoles] = createSignal<ReadonlySet<ToggleRole>>(new Set());

/** Reactive: whether the role's groups are shown (true when it has none). */
export function roleVisible(role: ToggleRole): boolean {
  return !hiddenRoles().has(role);
}

function groupsOf(dock: DockviewApi, role: GroupRole) {
  return dock.groups.filter((group) => groupRole(group.id) === role);
}

/** Re-reads visibility from the dock (after a layout load or group changes). */
export function syncVisibility(dock: DockviewApi): void {
  const hidden = new Set<ToggleRole>();
  for (const role of ['terminal', 'files'] as const) {
    const groups = groupsOf(dock, role);
    if (groups.length > 0 && groups.every((group) => !group.api.isVisible)) hidden.add(role);
  }
  setHiddenRoles(hidden);
}

/** Toggles the role's groups; false when there is no group (the caller opens a panel). */
export function toggleRoleGroups(dock: DockviewApi, role: ToggleRole): boolean {
  const groups = groupsOf(dock, role);
  if (groups.length === 0) return false;
  const show = groups.every((group) => !group.api.isVisible);
  for (const group of groups) group.api.setVisible(show);
  syncVisibility(dock);
  return true;
}

/** Shows a group again before a panel is added to it. */
export function revealGroup(dock: DockviewApi, groupId: string): void {
  const group = dock.getGroup(groupId);
  if (group && !group.api.isVisible) {
    group.api.setVisible(true);
    syncVisibility(dock);
  }
}
