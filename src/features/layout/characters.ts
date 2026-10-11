// Gives agent panels (terminal CLIs and API chats) their pixel character: auto-assigns an
// unused one to every panel that wants one (shells only when the setting says so) and
// changes it on request. The choice is stored in the panel's config, so it persists.
import type { DockviewApi, IDockviewPanel } from 'dockview-core';
import { createSignal } from 'solid-js';
import { loadCharacters, pickCharacter, type CharacterRef } from '../characters';
import { settings } from '../settings/settingsStore';
import { wantsCharacter } from '../terminal';
import { parsePanelParams, type PanelParams } from './panels';

type AgentParams = Extract<PanelParams, { kind: 'terminal' | 'chat' }>;

function agentPanels(dock: DockviewApi): { panel: IDockviewPanel; params: AgentParams }[] {
  return dock.panels.flatMap((panel) => {
    const params = parsePanelParams(panel.params);
    return params?.kind === 'terminal' || params?.kind === 'chat' ? [{ panel, params }] : [];
  });
}

function wants(params: AgentParams): boolean {
  return params.kind === 'chat' || wantsCharacter(params.config, settings.characters.onShells);
}

/** The character a panel shows (null for shells without the setting, or avatars off). */
export function characterOf(params: PanelParams | null): CharacterRef | null {
  if (!settings.characters.enabled) return null;
  if (params?.kind === 'chat') return params.config.character;
  if (params?.kind !== 'terminal' || !wants(params)) return null;
  return params.config.character;
}

export function usedCharacters(dock: DockviewApi): string[] {
  return agentPanels(dock).flatMap(({ params }) =>
    params.config.character ? [params.config.character.id] : [],
  );
}

function withCharacter(params: AgentParams, character: CharacterRef): AgentParams {
  return params.kind === 'chat'
    ? { ...params, config: { ...params.config, character } }
    : { ...params, config: { ...params.config, character } };
}

/** Assigns a character to every panel that wants one and has none (loads sprites first). */
export async function ensureCharacters(dock: DockviewApi): Promise<boolean> {
  const all = await loadCharacters();
  const used = usedCharacters(dock);
  let changed = false;
  for (const { panel, params } of agentPanels(dock)) {
    if (params.config.character || !wants(params)) continue;
    const character = pickCharacter(all, used);
    if (!character) continue;
    used.push(character.id);
    panel.api.updateParameters(withCharacter(params, character));
    changed = true;
  }
  return changed;
}

export function setCharacter(dock: DockviewApi, panelId: string, character: CharacterRef): boolean {
  const panel = dock.getPanel(panelId);
  const params = panel ? parsePanelParams(panel.params) : null;
  if (!panel || (params?.kind !== 'terminal' && params?.kind !== 'chat')) return false;
  panel.api.updateParameters(withCharacter(params, character));
  return true;
}

// The agent panel the Characters view assigns to: the last focused one that has a character.
const [focusedAgent, setFocusedAgent] = createSignal<{ id: string; title: string } | null>(null);
export { focusedAgent };

export function trackFocusedAgent(dock: DockviewApi): void {
  const active = dock.activePanel;
  const params = active ? parsePanelParams(active.params) : null;
  if (active && (params?.kind === 'terminal' || params?.kind === 'chat') && wants(params)) {
    setFocusedAgent({ id: active.id, title: active.title ?? active.id });
  } else if (focusedAgent() && !dock.getPanel(focusedAgent()?.id ?? '')) {
    setFocusedAgent(null);
  }
}
