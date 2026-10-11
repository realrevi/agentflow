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
