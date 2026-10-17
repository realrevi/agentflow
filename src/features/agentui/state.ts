// The state every agent panel's character shows (terminal heuristic or API stream), per
// panel id. Read by tabs, the Agents list and the panels themselves.
import { createStore, produce } from 'solid-js/store';
import type { AgentState } from '../characters/stateMachine';

const [states, setStates] = createStore<Record<string, AgentState>>({});

/** Reactive. Panels without activity yet are idle. */
export function agentState(panelId: string): AgentState {
  return states[panelId] ?? 'idle';
}

export function setAgentState(panelId: string, state: AgentState): void {
  if (states[panelId] !== state) setStates(panelId, state);
}

export function clearAgentState(panelId: string): void {
  setStates(
    produce((all) => {
      delete all[panelId];
    }),
  );
}

const FLASH_MS = 1600;
const flashes = new Map<string, { timer: number; before: AgentState }>();

/** Plays `state` for a moment (e.g. "error" on a budget alert), then returns to what the
 * panel showed before, unless its state changed meanwhile. */
export function flashAgentState(panelId: string, state: AgentState): void {
  const running = flashes.get(panelId);
  window.clearTimeout(running?.timer);
  const before = running?.before ?? agentState(panelId);
  setAgentState(panelId, state);
  const timer = window.setTimeout(() => {
    flashes.delete(panelId);
    if (agentState(panelId) === state) setAgentState(panelId, before);
  }, FLASH_MS);
  flashes.set(panelId, { timer, before });
}
