import type { DockviewApi } from 'dockview-core';
import { panelOptions } from './panels';

const WORKSPACE_HEIGHT_RATIO = 0.38;

/** Agent 1 | Agent 2 on top, Workspace across the bottom. */
export function buildDefaultLayout(dock: DockviewApi): void {
  const agent1 = panelOptions({ kind: 'agent', index: 1 });
  dock.addPanel(agent1);
  dock.addPanel({
    ...panelOptions({ kind: 'agent', index: 2 }),
    position: { referencePanel: agent1.id, direction: 'right' },
  });
  const height = Math.round(dock.height * WORKSPACE_HEIGHT_RATIO);
  dock.addPanel({
    ...panelOptions({ kind: 'workspace', index: 1 }),
    position: { direction: 'below' },
    ...(height > 0 ? { initialHeight: height } : {}),
  });
  dock.getPanel(agent1.id)?.api.setActive();
}
