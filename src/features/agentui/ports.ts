// How workstreams reach an agent panel: each open terminal agent or chat registers a port
// that sends text through its own prompt path (bracketed paste + Enter for CLIs, a chat send
// for API agents) with history, character states and the packet animation. Text sent here
// is data typed into the agent, never run by AgentFlow.
import { createSignal } from 'solid-js';

export interface AgentPort {
  kind: 'cli' | 'api';
  /** The agent can take a prompt now (CLI running, chat not streaming). */
  ready(): boolean;
  /** Sends `text`; false if it could not be sent. */
  deliver(text: string): boolean | Promise<boolean>;
  /** e.g. "Claude Code", the API model. */
  label(): string;
  /** The AgentFlow agent (profile) it runs, if any: managers see it in the worker list. */
  role?(): string | null;
}

const ports = new Map<string, AgentPort>();
// Bumped on (un)registration, so readers of `portOf` re-run.
const [version, setVersion] = createSignal(0);

export function registerPort(panelId: string, port: AgentPort): () => void {
  ports.set(panelId, port);
  setVersion((v) => v + 1);
  return () => {
    if (ports.get(panelId) === port) ports.delete(panelId);
    setVersion((v) => v + 1);
  };
}

/** Reactive. */
export function portOf(panelId: string): AgentPort | undefined {
  version();
  return ports.get(panelId);
}

// Extra system text per API panel: a workstream manager's instructions, appended to its
// system prompt while it manages (never stored with the conversation).
const extras = new Map<string, string>();

export function setExtraSystem(panelId: string, text: string | null): void {
  if (text) extras.set(panelId, text);
  else extras.delete(panelId);
}

export function extraSystemOf(panelId: string): string | null {
  return extras.get(panelId) ?? null;
}
