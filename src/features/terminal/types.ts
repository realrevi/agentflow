/** `stopped`: never started (or failed to start); `exited`: the process ended. */
export type TerminalStatus = 'stopped' | 'starting' | 'running' | 'exited';

/** Activity signals for the agent state heuristic (agentActivity.ts). */
export interface ActivityHooks {
  start(): void;
  output(): void;
  /** Enter typed directly into the terminal. */
  enter(): void;
  /** Any other keys typed into the terminal (their echo is not work). */
  typed(): void;
  exit(code: number | null): void;
}
