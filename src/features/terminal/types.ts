/** `stopped`: never started (or failed to start); `exited`: the process ended. */
export type TerminalStatus = 'stopped' | 'starting' | 'running' | 'exited';
