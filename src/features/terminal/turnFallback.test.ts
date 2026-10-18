import { describe, expect, it } from 'vitest';
import { LOG_GRACE_MS, TurnFallback } from './turnFallback';

function setup(logsTurns: boolean, match: 'exact' | 'draft' | null = null) {
  let now = 0;
  let timers: { at: number; run: () => void; id: number }[] = [];
  let nextId = 1;
  const state = { match, ended: 0 };
  const fallback = new TurnFallback(logsTurns, {
    logMatch: () => state.match,
    end: () => state.ended++,
    setTimer: (run, ms) => {
      const id = nextId++;
      timers.push({ at: now + ms, run, id });
      return id;
    },
    clearTimer: (id) => {
      timers = timers.filter((timer) => timer.id !== id);
    },
  });
  const advance = (ms: number) => {
    now += ms;
    const due = timers.filter((timer) => timer.at <= now);
    timers = timers.filter((timer) => timer.at > now);
    for (const timer of due) timer.run();
  };
  return { fallback, state, advance };
}

describe('terminal idle turn fallback', () => {
  it('ends a turn when the terminal goes quiet after a prompt (no turn-end log)', () => {
    const { fallback, state } = setup(false);
    fallback.state('done'); // startup output, no prompt yet
    expect(state.ended).toBe(0);
    fallback.prompt();
    fallback.state('think');
    fallback.state('work');
    fallback.state('done');
    expect(state.ended).toBe(1);
    fallback.state('idle');
    fallback.state('done');
    expect(state.ended, 'once per prompt').toBe(1);
  });

  it('gives CLIs with a turn-end log time to report first', () => {
    const { fallback, state, advance } = setup(true);
    fallback.prompt();
    fallback.state('done');
    advance(LOG_GRACE_MS - 1);
    expect(state.ended).toBe(0);
    fallback.logEnded();
    advance(LOG_GRACE_MS);
    expect(state.ended, 'the log ended it').toBe(0);

    fallback.prompt();
    fallback.state('done');
    advance(LOG_GRACE_MS);
    expect(state.ended, 'no log: the heuristic ends it').toBe(1);
  });

  it('output resuming cancels a pending end; an exact log never falls back', () => {
    const { fallback, state, advance } = setup(true);
    fallback.prompt();
    fallback.state('done');
    fallback.state('work');
    advance(LOG_GRACE_MS * 2);
    expect(state.ended).toBe(0);

    state.match = 'exact';
    fallback.state('done');
    advance(LOG_GRACE_MS * 2);
    expect(state.ended).toBe(0);
  });

  it('an exit forgets the prompt', () => {
    const { fallback, state } = setup(false);
    fallback.prompt();
    fallback.state('sleep');
    fallback.state('done');
    expect(state.ended).toBe(0);
  });
});
