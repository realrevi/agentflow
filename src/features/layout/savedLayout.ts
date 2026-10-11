// The persisted layout: dockview's JSON plus the group roles, under a format version.
// Version 1 (phases 1–2) was dockview's JSON alone, with the old Workspace placeholder;
// anything that is not the current version is replaced by the new default layout.
import type { SerializedDockview } from 'dockview-core';
import type { SerializedLayout } from '../../shared/types';
import { isGroupRole, type GroupRole } from './roles';

export const LAYOUT_VERSION = 2;

export interface SavedLayout {
  dock: SerializedDockview;
  roles: Record<string, GroupRole>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** `null` means "build the default layout" (nothing saved, an older version, or junk). */
export function parseSavedLayout(raw: SerializedLayout | null): SavedLayout | null {
  if (!raw || raw['version'] !== LAYOUT_VERSION) return null;
  const { dock, roles } = raw;
  if (!isRecord(dock) || !isRecord(dock['grid']) || !isRecord(dock['panels'])) return null;
  const validRoles = Object.entries(isRecord(roles) ? roles : {}).filter(
    (entry): entry is [string, GroupRole] => isGroupRole(entry[1]),
  );
  // dockview validates its own JSON in fromJSON; here it is opaque data.
  return { dock: dock as unknown as SerializedDockview, roles: Object.fromEntries(validRoles) };
}

export function serializeLayout(
  dock: SerializedDockview,
  roles: Record<string, GroupRole>,
): SerializedLayout {
  return { version: LAYOUT_VERSION, dock: dock as unknown as Record<string, unknown>, roles };
}
