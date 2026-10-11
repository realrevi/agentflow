import type { DockviewApi } from 'dockview-core';
import { newTerminalConfig } from '../terminal';
import { panelOptions } from './panels';
import { createRoleGroup } from './placement';

/**
 * The center agent area (empty: its watermark offers the agent CLIs), the file tree on
 * the right and a shell at the bottom, spanning the center.
 */
export function buildDefaultLayout(dock: DockviewApi): void {
  const agents = createRoleGroup(dock, 'agents');
  const files = createRoleGroup(dock, 'files');
  const terminal = createRoleGroup(dock, 'terminal');
  dock.addPanel({ ...panelOptions({ kind: 'files' }), position: { referenceGroup: files.id } });
  dock.addPanel({
    ...panelOptions({ kind: 'terminal', index: 1, config: newTerminalConfig('shell') }),
    position: { referenceGroup: terminal.id },
  });
  agents.api.setActive();
}
