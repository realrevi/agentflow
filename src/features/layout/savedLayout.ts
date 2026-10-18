// The persisted layout: dockview's JSON plus the group roles, the workstreams and the
// Stage's character positions, under a format version. Version 1 (phases 1–2) was
// dockview's JSON alone, with the old Workspace placeholder, and is replaced by the default
// layout; version 2 (phases 3–8) had no workstreams and is upgraded as it is.
import type { SerializedDockview } from 'dockview-core';
import type { SerializedLayout } from '../../shared/types';
import { isGroupRole, type GroupRole } from './roles';

export const LAYOUT_VERSION = 3;

export interface LayoutExtras {
  /** Opaque here: parsed by features/workstreams. */
  workstreams: unknown[];
  /** Opaque here: parsed by features/stage. */
  stage: Record<string, unknown>;
}

export interface SavedLayout extends LayoutExtras {
  dock: SerializedDockview;
  roles: Record<string, GroupRole>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** `null` means "build the default layout" (nothing saved, a version 1 layout, or junk). */
export function parseSavedLayout(raw: SerializedLayout | null): SavedLayout | null {
  const version = raw?.['version'];
  if (!raw || (version !== 2 && version !== LAYOUT_VERSION)) return null;
  const { dock, roles, workstreams, stage } = raw;
  if (!isRecord(dock) || !isRecord(dock['grid']) || !isRecord(dock['panels'])) return null;
  const validRoles = Object.entries(isRecord(roles) ? roles : {}).filter(
    (entry): entry is [string, GroupRole] => isGroupRole(entry[1]),
  );
  // dockview validates its own JSON in fromJSON; here it is opaque data.
  return {
    dock: dock as unknown as SerializedDockview,
    roles: Object.fromEntries(validRoles),
    workstreams: Array.isArray(workstreams) ? workstreams : [],
    stage: isRecord(stage) ? stage : {},
  };
}

export function serializeLayout(
  dock: SerializedDockview,
  roles: Record<string, GroupRole>,
  extras: LayoutExtras = { workstreams: [], stage: {} },
): SerializedLayout {
  return {
    version: LAYOUT_VERSION,
    dock: dock as unknown as Record<string, unknown>,
    roles,
    workstreams: extras.workstreams,
    stage: extras.stage,
  };
}
