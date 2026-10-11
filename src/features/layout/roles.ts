// Every dock group has a role that decides which new panels open in it: agent CLIs,
// editors and settings in the center ("agents"), the file tree on the right ("files"),
// plain shells at the bottom ("terminal"). Roles persist with the layout.
import { createStore, produce, unwrap } from 'solid-js/store';
import type { PanelParams } from './panels';

export type GroupRole = 'agents' | 'files' | 'terminal';

const ROLES: readonly GroupRole[] = ['agents', 'files', 'terminal'];

export function isGroupRole(value: unknown): value is GroupRole {
  return typeof value === 'string' && (ROLES as readonly string[]).includes(value);
}

/** Where a panel opens by default. */
export function roleForPanel(params: PanelParams): GroupRole {
  if (params.kind === 'files') return 'files';
  if (params.kind === 'terminal' && params.config.kind === 'shell') return 'terminal';
  return 'agents';
}

// Reactive, so a group's watermark updates when its role is assigned.
const [roles, setRoles] = createStore<Record<string, GroupRole>>({});
const lastFocused = new Map<GroupRole, string>();

export function groupRole(groupId: string): GroupRole | undefined {
  return roles[groupId];
}

export function setGroupRole(groupId: string, role: GroupRole): void {
  setRoles(groupId, role);
}

export function forgetGroup(groupId: string): void {
  setRoles(
    produce((all) => {
      delete all[groupId];
    }),
  );
  for (const [role, id] of lastFocused) if (id === groupId) lastFocused.delete(role);
}

export function noteGroupFocus(groupId: string): void {
  const role = roles[groupId];
  if (role) lastFocused.set(role, groupId);
}

export function lastFocusedGroup(role: GroupRole): string | undefined {
  return lastFocused.get(role);
}

export function replaceRoles(next: Record<string, GroupRole>): void {
  lastFocused.clear();
  setRoles(produce((all) => Object.assign(clear(all), next)));
}

/** A plain copy for persisting. */
export function snapshotRoles(): Record<string, GroupRole> {
  return { ...unwrap(roles) };
}

function clear(all: Record<string, GroupRole>): Record<string, GroupRole> {
  for (const id of Object.keys(all)) delete all[id];
  return all;
}
