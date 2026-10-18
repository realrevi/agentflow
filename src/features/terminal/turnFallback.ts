// The terminal idle heuristic as a turn detector, for CLIs whose log marks no turn end
// (Gemini, custom commands) or whose log was not found. A turn ends when the character goes
// "done" (2 s without output) after a prompt; the result is marked approximate. CLIs that
// log turn ends (Claude Code, Codex) wait a little longer for their log first.
import type { AgentState } from '../characters/stateMachine';

/** How long a CLI with a turn-end log gets to report before the heuristic does. */
export const LOG_GRACE_MS = 3000;

export interface FallbackDeps {
  /** 'exact' once the panel's log reports turn ends. */
  logMatch(): 'exact' | 'draft' | null;
  /** Ends the turn (idle source). */
  end(): void;
  setTimer(run: () => void, ms: number): number;
  clearTimer(id: number): void;
}

export class TurnFallback {
  private prompted = false;
  private timer: number | undefined;

  /** `logsTurns`: the CLI writes turn ends to its log (Claude Code, Codex). */
  constructor(
    private readonly logsTurns: boolean,
    private readonly deps: FallbackDeps,
  ) {}

  /** A prompt was sent (bar, delivery or Enter in the terminal). */
  prompt(): void {
    this.prompted = true;
    this.cancel();
  }

  /** The character's state changed. */
  state(next: AgentState): void {
    if (next === 'work' || next === 'think') {
      this.cancel();
      return;
    }
    if (next === 'sleep' || next === 'error') {
      this.prompted = false;
      this.cancel();
      return;
    }
    if (next !== 'done' || !this.prompted || this.timer !== undefined) return;
    if (this.deps.logMatch() === 'exact') return; // the log ends this turn
    const fire = () => {
      this.timer = undefined;
      if (!this.prompted || this.deps.logMatch() === 'exact') return;
      this.prompted = false;
      this.deps.end();
    };
    if (!this.logsTurns) {
      fire();
      return;
    }
    this.timer = this.deps.setTimer(fire, LOG_GRACE_MS);
  }

  /** The log reported the end of the turn. */
  logEnded(): void {
    this.prompted = false;
    this.cancel();
  }

  dispose(): void {
    this.cancel();
  }

  private cancel(): void {
    if (this.timer === undefined) return;
    this.deps.clearTimer(this.timer);
    this.timer = undefined;
  }
}
