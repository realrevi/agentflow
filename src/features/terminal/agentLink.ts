// Phase 9 wiring of one terminal agent panel: its delivery port (a handoff goes through the
// prompt bar like a typed prompt), turn ends (the CLI log, else the idle heuristic, marked
// approximate), the bubble activity when no log tells more ("Working…"), user typing and
// the attention glow on a failed exit. Call inside the panel's component.
import { createEffect, on, onCleanup } from 'solid-js';
import {
  agentState,
  cliStarted,
  idleTurnEnded,
  kindText,
  logMatch,
  logToolPending,
  onTurnEnded,
  registerPort,
  requestAttention,
  setActivity,
  clearActivity,
} from '../agentui';
import type { AgentState } from '../characters/stateMachine';
import { t } from '../i18n';
import { TurnFallback } from './turnFallback';

/** Reports are cut far below this anyway; the screen text is only a fallback. */
const SCREEN_LINES = 400;

export interface TerminalLinkOptions {
  panelId: string;
  /** Claude Code and Codex log their turn ends. */
  logsTurns: boolean;
  running: () => boolean;
  /** Sends through the prompt bar (history, packet), false if not possible. */
  deliver: (text: string) => boolean;
  label: () => string;
  screenText: (maxLines: number) => string;
}

/** What `adjust` of the panel's AgentActivity should use. */
export function cliAdjust(panelId: string) {
  return (state: AgentState): AgentState =>
    (state === 'done' || state === 'idle') &&
    logMatch(panelId) === 'exact' &&
    logToolPending(panelId)
      ? 'wait'
      : state;
}

export function linkTerminal(options: TerminalLinkOptions) {
  const { panelId } = options;
  const fallback = new TurnFallback(options.logsTurns, {
    logMatch: () => logMatch(panelId),
    end: () => {
      idleTurnEnded(panelId, options.screenText(SCREEN_LINES));
      setActivity(panelId, 'done', kindText('done'), true);
    },
    setTimer: (run, ms) => window.setTimeout(run, ms),
    clearTimer: (id) => window.clearTimeout(id),
  });
  onCleanup(
    registerPort(panelId, {
      kind: 'cli',
      ready: options.running,
      deliver: options.deliver,
      label: options.label,
    }),
  );
  onCleanup(
    onTurnEnded((end) => {
      if (end.panel === panelId && end.source === 'log') fallback.logEnded();
    }),
  );
  // The heuristic's states drive the fallback, and the bubble while no log matched.
  createEffect(
    on(
      () => agentState(panelId),
      (state) => {
        fallback.state(state);
        if (logMatch(panelId) !== null && state !== 'error' && state !== 'sleep') return;
        if (state === 'work') setActivity(panelId, 'think', t('doing.work'), true);
        else if (state === 'think') setActivity(panelId, 'think', kindText('think'), true);
        else if (state === 'error') setActivity(panelId, 'error', kindText('error'), true);
        else if (state === 'idle' || state === 'sleep') setActivity(panelId, 'idle', '', true);
      },
    ),
  );
  onCleanup(() => {
    fallback.dispose();
    clearActivity(panelId);
  });

  return {
    /** A prompt went to the CLI. */
    prompt: () => fallback.prompt(),
    /** The process (re)started: its log is matched again. */
    started: () => cliStarted(panelId),
    exited: (code: number | null) => {
      if (code !== 0 && code !== null) requestAttention(panelId, 'error', options.label());
    },
  };
}
