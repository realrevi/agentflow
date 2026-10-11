// Status of every open terminal panel, keyed by panel id. Drives the tab status dot.
import { createStore, produce } from 'solid-js/store';
import type { TerminalStatus } from './types';

const [statuses, setStatuses] = createStore<Record<string, TerminalStatus>>({});

export function terminalStatus(panelId: string): TerminalStatus {
  return statuses[panelId] ?? 'stopped';
}

export function setTerminalStatus(panelId: string, status: TerminalStatus): void {
  setStatuses(panelId, status);
}

export function clearTerminalStatus(panelId: string): void {
  setStatuses(
    produce((all) => {
      delete all[panelId];
    }),
  );
}
