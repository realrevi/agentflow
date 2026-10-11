import type { DockviewApi } from 'dockview-core';
import { newTerminalConfig } from '../terminal';
import { panelOptions } from './panels';

const WORKSPACE_HEIGHT_RATIO = 0.38;

/** Two shell terminals side by side on top, Workspace across the bottom. */
export function buildDefaultLayout(dock: DockviewApi): void {
  const first = panelOptions({ kind: 'terminal', index: 1, config: newTerminalConfig('shell') });
  dock.addPanel(first);
  dock.addPanel({
    ...panelOptions({ kind: 'terminal', index: 2, config: newTerminalConfig('shell') }),
    position: { referencePanel: first.id, direction: 'right' },
  });
  const height = Math.round(dock.height * WORKSPACE_HEIGHT_RATIO);
  dock.addPanel({
    ...panelOptions({ kind: 'workspace' }),
    position: { direction: 'below' },
    ...(height > 0 ? { initialHeight: height } : {}),
  });
  dock.getPanel(first.id)?.api.setActive();
}
