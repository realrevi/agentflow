// Drives a terminal panel's agent state through the heuristic in
// characters/stateMachine.ts. One timer per panel at most (no per-chunk timer churn).
import { clearAgentState, setAgentState } from '../agentui/state';
import { AgentStateTracker, type AgentState } from '../characters/stateMachine';

export class AgentActivity {
  private readonly tracker = new AgentStateTracker();
  private timer: number | undefined;
  private timerAt = 0;

  constructor(private readonly panelId: string) {}

  start(): void {
    this.apply(this.tracker.start());
  }

  prompt(): void {
    this.apply(this.tracker.prompt(performance.now()));
  }

  typed(): void {
    this.tracker.typed(performance.now());
  }

  output(): void {
    const before = this.tracker.state;
    const state = this.tracker.output(performance.now());
    if (state !== before) this.apply(state);
    else this.arm();
  }

  exit(code: number | null): void {
    this.apply(this.tracker.exit(code));
  }

  dispose(): void {
    window.clearTimeout(this.timer);
    this.timer = undefined;
    clearAgentState(this.panelId);
  }

  private apply(state: AgentState): void {
    setAgentState(this.panelId, state);
    this.arm();
  }

  /** Keeps one timer for the next deadline; a later deadline is picked up when it fires. */
  private arm(): void {
    const deadline = this.tracker.nextDeadline();
    if (deadline === null) {
      window.clearTimeout(this.timer);
      this.timer = undefined;
      return;
    }
    if (this.timer !== undefined && this.timerAt <= deadline) return;
    window.clearTimeout(this.timer);
    this.timerAt = deadline;
    this.timer = window.setTimeout(
      () => {
        this.timer = undefined;
        this.apply(this.tracker.tick(performance.now()));
      },
      Math.max(0, deadline - performance.now()),
    );
  }
}
