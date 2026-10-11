// Where groups go: the role groups of the default layout, recreated at their default
// position when a panel needs one that was closed.
import type { DockviewApi, DockviewGroupPanel, IDockviewGroupPanel } from 'dockview-core';
import { groupRole, lastFocusedGroup, setGroupRole, type GroupRole } from './roles';

/** Files on the right at ~22% width; terminals under the center at ~30% height. */
export const FILES_WIDTH = 0.22;
export const TERMINAL_HEIGHT = 0.3;

function findRole(dock: DockviewApi, role: GroupRole): DockviewGroupPanel | undefined {
  return dock.groups.find((group) => groupRole(group.id) === role);
}

/** The last focused group of `role`, any group of it, or a new one at its default spot. */
export function groupForRole(dock: DockviewApi, role: GroupRole): IDockviewGroupPanel {
  const remembered = lastFocusedGroup(role);
  const group = (remembered ? dock.getGroup(remembered) : undefined) ?? findRole(dock, role);
  return group ?? createRoleGroup(dock, role);
}

export function createRoleGroup(dock: DockviewApi, role: GroupRole): DockviewGroupPanel {
  const group = addGroupAt(dock, role);
  setGroupRole(group.id, role);
  if (role === 'files') resize(group, { width: dock.width * FILES_WIDTH });
  const terminal = role === 'files' ? undefined : findRole(dock, 'terminal');
  if (terminal) resize(terminal, { height: dock.height * TERMINAL_HEIGHT });
  return group;
}

function addGroupAt(dock: DockviewApi, role: GroupRole): DockviewGroupPanel {
  if (dock.groups.length === 0) return dock.addGroup();
  if (role === 'files') return dock.addGroup({ direction: 'right' });
  if (role === 'terminal') {
    const agents = findRole(dock, 'agents');
    // Under the center only, so the file tree keeps its full height.
    return agents
      ? dock.addGroup({ referenceGroup: agents, direction: 'below' })
      : dock.addGroup({ direction: 'below' });
  }
  const terminal = findRole(dock, 'terminal');
  if (terminal) return dock.addGroup({ referenceGroup: terminal, direction: 'above' });
  const files = findRole(dock, 'files');
  if (files) return dock.addGroup({ referenceGroup: files, direction: 'left' });
  return dock.addGroup({ direction: 'left' });
}

function resize(group: DockviewGroupPanel, size: { width?: number; height?: number }): void {
  const width = Math.round(size.width ?? 0);
  const height = Math.round(size.height ?? 0);
  if (width > 0) group.api.setSize({ width });
  if (height > 0) group.api.setSize({ height });
}
