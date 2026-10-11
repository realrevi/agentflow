// The agent state heuristic (see CLAUDE.md). Pure and time-injected, so it is easy to test:
//   prompt sent -> think; output arriving -> work; 2 s without output -> done;
//   5 s after done -> idle; exit 0 -> sleep; other exits -> error; (re)start -> idle.
// Output within 400 ms of a prompt or of keys typed into the terminal is just the echo
// and is ignored (unless the agent is already working).

export type AgentState = 'idle' | 'think' | 'work' | 'done' | 'sleep' | 'error';

export const QUIET_MS = 2000;
export const REST_MS = 5000;
export const ECHO_MS = 400;

export class AgentStateTracker {
  state: AgentState = 'idle';
  private lastOutput = 0;
  private echoUntil = Number.NEGATIVE_INFINITY;
  private doneAt = 0;

  start(): AgentState {
    this.state = 'idle';
    return this.state;
  }

  prompt(now: number): AgentState {
    if (this.state === 'sleep' || this.state === 'error') return this.state;
    this.echoUntil = now + ECHO_MS;
    this.state = 'think';
    return this.state;
  }

  /** Keys typed straight into the terminal: what comes back right away is their echo. */
  typed(now: number): void {
    this.echoUntil = now + ECHO_MS;
  }

  output(now: number): AgentState {
    if (this.state === 'sleep' || this.state === 'error') return this.state;
    if (this.state !== 'work' && now < this.echoUntil) return this.state;
    this.lastOutput = now;
    this.state = 'work';
    return this.state;
  }

  exit(code: number | null): AgentState {
    this.state = code === 0 ? 'sleep' : 'error';
    return this.state;
  }

  /** Applies the quiet/rest timeouts. */
  tick(now: number): AgentState {
    if (this.state === 'work' && now - this.lastOutput >= QUIET_MS) {
      this.state = 'done';
      this.doneAt = now;
    }
    if (this.state === 'done' && now - this.doneAt >= REST_MS) this.state = 'idle';
    return this.state;
  }

  /** When `tick` may change the state next, or null if nothing is pending. */
  nextDeadline(): number | null {
    if (this.state === 'work') return this.lastOutput + QUIET_MS;
    if (this.state === 'done') return this.doneAt + REST_MS;
    return null;
  }
}
