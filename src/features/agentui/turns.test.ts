import { describe, expect, it } from 'vitest';
import { activityOf } from './activity';
import { interactionClears, shouldNotify } from './attention';
import {
  applyTurnEvent,
  cliStarted,
  idleTurnEnded,
  logMatch,
  logToolPending,
  onTurnEnded,
  type TurnEnd,
} from './turns';

function collect(): { ends: TurnEnd[]; stop: () => void } {
  const ends: TurnEnd[] = [];
  const stop = onTurnEnded((end) => ends.push(end));
  return { ends, stop };
}

describe('turn feed', () => {
  it('reports exact log turn ends, numbered per panel, with live activity', () => {
    const { ends, stop } = collect();
    applyTurnEvent({ type: 'attached', panel: 'terminal-1', exact: true });
    applyTurnEvent({
      type: 'activity',
      panel: 'terminal-1',
      kind: 'tool',
      action: 'run',
      target: 'npm test',
    });
    expect(activityOf('terminal-1')?.text).toBe('Running npm test');
    expect(logToolPending('terminal-1')).toBe(true);
    applyTurnEvent({
      type: 'turnEnded',
      panel: 'terminal-1',
      report: 'Fixed.',
      interrupted: false,
    });
    applyTurnEvent({ type: 'turnEnded', panel: 'terminal-1', report: '', interrupted: true });
    stop();
    expect(ends.map((e) => [e.turn, e.report, e.source, e.interrupted, e.approximate])).toEqual([
      [1, 'Fixed.', 'log', false, false],
      [2, '', 'log', true, false],
    ]);
    expect(logToolPending('terminal-1')).toBe(false);
    // The idle heuristic never ends a turn the log reports.
    expect(idleTurnEnded('terminal-1', 'screen')).toBeNull();
  });

  it('the idle fallback is marked approximate and uses the log draft when there is one', () => {
    const { ends, stop } = collect();
    applyTurnEvent({ type: 'attached', panel: 'terminal-2', exact: false });
    applyTurnEvent({ type: 'draft', panel: 'terminal-2', text: 'All tests pass.' });
    // Without exact ends the log's turnEnded is ignored: the heuristic ends the turn.
    applyTurnEvent({ type: 'turnEnded', panel: 'terminal-2', report: 'x', interrupted: false });
    expect(ends).toHaveLength(0);
    applyTurnEvent({ type: 'draft', panel: 'terminal-2', text: 'All tests pass.' });
    idleTurnEnded('terminal-2', 'screen text');
    idleTurnEnded('terminal-3', '  only the screen  ');
    stop();
    expect(ends.map((e) => [e.panel, e.report, e.source, e.approximate])).toEqual([
      ['terminal-2', 'All tests pass.', 'idle', true],
      ['terminal-3', 'only the screen', 'idle', true],
    ]);
  });

  it('a restarted CLI has to be matched again', () => {
    applyTurnEvent({ type: 'attached', panel: 'terminal-4', exact: true });
    expect(logMatch('terminal-4')).toBe('exact');
    cliStarted('terminal-4');
    expect(logMatch('terminal-4')).toBeNull();
  });
});

describe('attention rules', () => {
  it('clears on a click or typing, on scrolling only when the panel is focused', () => {
    expect(interactionClears('pointerdown', false)).toBe(true);
    expect(interactionClears('keydown', true)).toBe(true);
    expect(interactionClears('wheel', true)).toBe(true);
    expect(interactionClears('wheel', false)).toBe(false);
    expect(interactionClears('pointerenter', true)).toBe(false);
    expect(interactionClears('mousemove', true)).toBe(false);
  });

  it('notifies only when enabled, in the background, and not too often', () => {
    expect(shouldNotify(false, false, 0, 100_000)).toBe(false);
    expect(shouldNotify(true, true, 0, 100_000)).toBe(false);
    expect(shouldNotify(true, false, 0, 100_000)).toBe(true);
    expect(shouldNotify(true, false, 95_000, 100_000)).toBe(false);
  });
});
