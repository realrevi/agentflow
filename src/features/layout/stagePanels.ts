// The Stage and Timeline panels: the Stage opens as the first tab of the center (on its own
// when the first workstream is created, without taking focus) and moves between the center
// and a group on the left; the Timeline shows one workstream, optionally one pair of panels.
import type { DockviewGroupPanel } from 'dockview-core';
import { loadCharacters } from '../characters/catalog';
import { addPanel, currentDock } from './layoutStore';
import { ensureCharacters } from './characters';
import { panelOptions, parsePanelParams } from './panels';
import { groupForRole } from './placement';
import { groupRole, setGroupRole } from './roles';

const STAGE_WIDTH = 0.3;

/** Whether the Stage sits in its own group on the left (else in the center). */
export function stageIsLeft(): boolean {
  const group = currentDock()?.getPanel('stage')?.group;
  return Boolean(group && groupRole(group.id) === 'stage');
}

/** Opens (or focuses) the Stage; `quiet` adds it without activating it. */
export function openStage(quiet = false): void {
  const dock = currentDock();
  if (!dock) return;
  const existing = dock.getPanel('stage');
  if (existing) {
    if (!quiet) existing.api.setActive();
    return;
  }
  const center = groupForRole(dock, 'agents');
  dock.addPanel({
    ...panelOptions({ kind: 'stage' }),
    position: { referenceGroup: center.id, index: 0 },
    inactive: quiet && center.panels.length > 0,
  });
}

export function moveStage(target: 'left' | 'center'): void {
  const dock = currentDock();
  const panel = dock?.getPanel('stage');
  if (!dock || !panel) return;
  const center = groupForRole(dock, 'agents') as DockviewGroupPanel;
  if (target === 'center') {
    if (panel.group.id !== center.id) panel.api.moveTo({ group: center, index: 0 });
  } else if (groupRole(panel.group.id) !== 'stage') {
    panel.api.moveTo({ group: center, position: 'left' });
    setGroupRole(panel.group.id, 'stage');
    panel.group.api.setSize({ width: Math.round(dock.width * STAGE_WIDTH) });
  }
  panel.api.setActive();
}

/** Opens the Timeline on `workstream` (and a pair of its panels, if given). */
export function openTimeline(workstream: string | null, pair: [string, string] | null = null) {
  const dock = currentDock();
  if (!dock) return;
  const params = { kind: 'timeline' as const, workstream, pair };
  const existing = dock.getPanel('timeline');
  if (existing) {
    existing.api.updateParameters(params);
    existing.api.setActive();
    return;
  }
  addPanel(dock, params);
}

/** The pixel character's name of a panel (once assigned), e.g. "Fox". */
export async function panelCharacterName(panelId: string): Promise<string | null> {
  const dock = currentDock();
  if (!dock) return null;
  const sheets = await loadCharacters();
  await ensureCharacters(dock);
  const params = parsePanelParams(dock.getPanel(panelId)?.params);
  const ref =
    params?.kind === 'terminal' || params?.kind === 'chat' ? params.config.character : null;
  return sheets.find((sheet) => sheet.id === ref?.id)?.name ?? null;
}
