// "New workstream" can be asked for from the + menu, the sidebar, the Stage and the command
// palette; the app renders the dialog (it composes the panels a workstream may create).
import { createSignal } from 'solid-js';

export interface NewWorkstreamPreset {
  manager?: string;
  workers?: string[];
}

const [request, setRequest] = createSignal<NewWorkstreamPreset | null>(null);

export const newWorkstreamRequest = request;

export function openNewWorkstream(preset: NewWorkstreamPreset = {}): void {
  setRequest({ ...preset });
}

export function closeNewWorkstream(): void {
  setRequest(null);
}
