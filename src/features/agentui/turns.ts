// Turn ends of every agent panel, numbered per panel: API chats report their stream's
// stop, CLI panels the end their session log marks (Claude Code, Codex), and CLIs without
// one the terminal idle heuristic (marked approximate; the report is the log's last
// assistant text or the terminal text). Also feeds the CLI log activity into activity.ts.
import { ipc } from '../../shared/ipc';
import type { TurnEvent } from '../../shared/types';
import { reportError } from '../i18n';
import { kindText, setActivity, toolText } from './activity';

export type TurnSource = 'stop' | 'log' | 'idle';

export interface TurnEnd {
  panel: string;
  /** 1, 2, ... per panel since the app started. */
  turn: number;
  /** The final assistant message (empty if interrupted). */
  report: string;
  source: TurnSource;
  interrupted: boolean;
  /** From the idle heuristic. */
  approximate: boolean;
  at: number;
}

type Listener = (end: TurnEnd) => void;

const listeners = new Set<Listener>();
const counts = new Map<string, number>();

interface LogState {
  exact: boolean;
  /** Assistant text so far (logs without exact ends). */
  draft: string;
  /** A tool call started and has not finished. */
  toolPending: boolean;
}

const logs = new Map<string, LogState>();

export function onTurnEnded(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function reportTurnEnded(
  panel: string,
  end: { report: string; source: TurnSource; interrupted?: boolean },
): TurnEnd {
  const turn = (counts.get(panel) ?? 0) + 1;
  counts.set(panel, turn);
  const event: TurnEnd = {
    panel,
    turn,
    report: end.report.trim(),
    source: end.source,
    interrupted: end.interrupted ?? false,
    approximate: end.source === 'idle',
    at: Date.now(),
  };
  for (const listener of [...listeners]) listener(event);
  return event;
}

/** A CLI process (re)starts in `panel`: its next log has to be matched again. */
export function cliStarted(panel: string): void {
  logs.delete(panel);
}

/** 'exact': the log reports turn ends; 'draft': only text; null: no log matched yet. */
export function logMatch(panel: string): 'exact' | 'draft' | null {
  const log = logs.get(panel);
  return log ? (log.exact ? 'exact' : 'draft') : null;
}

/** A tool call is running (or waiting for approval: the log cannot tell). */
export function logToolPending(panel: string): boolean {
  return logs.get(panel)?.toolPending ?? false;
}

/**
 * The terminal went quiet after a prompt. Ends the turn unless the panel's log reports
 * turn ends itself.
 */
export function idleTurnEnded(panel: string, screenText: string): TurnEnd | null {
  const log = logs.get(panel);
  if (log?.exact) return null;
  const report = log?.draft.trim() || screenText;
  if (log) log.draft = '';
  return reportTurnEnded(panel, { report, source: 'idle' });
}

/** Applies one event of the backend's turn Channel. Exported for tests. */
export function applyTurnEvent(event: TurnEvent): void {
  const panel = event.panel;
  const log = logs.get(panel) ?? { exact: false, draft: '', toolPending: false };
  switch (event.type) {
    case 'attached':
      logs.set(panel, { ...log, exact: event.exact });
      return;
    case 'activity':
      logs.set(panel, { ...log, toolPending: event.kind === 'tool' });
      if (event.kind === 'tool') setActivity(panel, 'tool', toolText(event.action, event.target));
      else setActivity(panel, event.kind, kindText(event.kind));
      return;
    case 'draft':
      logs.set(panel, { ...log, draft: event.text });
      return;
    case 'turnEnded':
      logs.set(panel, { ...log, draft: '', toolPending: false });
      if (!log.exact) return; // the idle heuristic ends this turn
      setActivity(
        panel,
        event.interrupted ? 'idle' : 'done',
        event.interrupted ? '' : kindText('done'),
      );
      reportTurnEnded(panel, {
        report: event.report,
        source: 'log',
        interrupted: event.interrupted,
      });
  }
}

let started: Promise<void> | undefined;

/** Subscribes to the backend's turn Channel once (app start). */
export function startTurnFeed(): Promise<void> {
  started ??= ipc
    .turnsSubscribe(applyTurnEvent)
    .then(() => undefined)
    .catch((err: unknown) => reportError('error.turns', err));
  return started;
}
