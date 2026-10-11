// Gives terminal panels their pixel character: auto-assigns an unused one to every agent
// panel (shells only when the setting says so) and changes it on request. The choice is
// stored in the panel's terminal config, so it persists with the layout.
import type { DockviewApi, IDockviewPanel } from 'dockview-core';
import { createSignal } from 'solid-js';
import { loadCharacters, pickCharacter, type CharacterRef } from '../characters';
import { settings } from '../settings/settingsStore';
import { wantsCharacter } from '../terminal';
import { parsePanelParams, type PanelParams } from './panels';

type TerminalParams = Extract<PanelParams, { kind: 'terminal' }>;

function terminals(dock: DockviewApi): { panel: IDockviewPanel; params: TerminalParams }[] {
  return dock.panels.flatMap((panel) => {
    const params = parsePanelParams(panel.params);
    return params?.kind === 'terminal' ? [{ panel, params }] : [];
  });
}

export function usedCharacters(dock: DockviewApi): string[] {
  return terminals(dock).flatMap(({ params }) =>
    params.config.character ? [params.config.character.id] : [],
  );
}

/** Assigns a character to every panel that wants one and has none (loads sprites first). */
export async function ensureCharacters(dock: DockviewApi): Promise<boolean> {
  const all = await loadCharacters();
  const used = usedCharacters(dock);
  let changed = false;
  for (const { panel, params } of terminals(dock)) {
    const config = params.config;
    if (config.character || !wantsCharacter(config, settings.characters.onShells)) continue;
    const character = pickCharacter(all, used);
    if (!character) continue;
    used.push(character.id);
    panel.api.updateParameters({ ...params, config: { ...config, character } });
    changed = true;
  }
  return changed;
}

export function setCharacter(dock: DockviewApi, panelId: string, character: CharacterRef): boolean {
  const panel = dock.getPanel(panelId);
  const params = panel ? parsePanelParams(panel.params) : null;
  if (!panel || params?.kind !== 'terminal') return false;
  panel.api.updateParameters({ ...params, config: { ...params.config, character } });
  return true;
}

// The agent panel the Characters view assigns to: the last focused one that has a character.
const [focusedAgent, setFocusedAgent] = createSignal<{ id: string; title: string } | null>(null);
export { focusedAgent };

export function trackFocusedAgent(dock: DockviewApi): void {
  const active = dock.activePanel;
  const params = active ? parsePanelParams(active.params) : null;
  if (
    active &&
    params?.kind === 'terminal' &&
    wantsCharacter(params.config, settings.characters.onShells)
  ) {
    setFocusedAgent({ id: active.id, title: active.title ?? active.id });
  } else if (focusedAgent() && !dock.getPanel(focusedAgent()?.id ?? '')) {
    setFocusedAgent(null);
  }
}
