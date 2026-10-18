// What every agent panel is doing right now, as a short localized text ("Running npm
// test", "Waiting for your approval"): from the CLI session logs, the API agent stream, or
// the terminal heuristic ("Working…", approximate). Read by the Stage bubbles.
import { createStore, produce } from 'solid-js/store';
import type { ToolAction } from '../../shared/types';
import { t, type I18nKey } from '../i18n';

export type ActivityKind = 'think' | 'tool' | 'wait' | 'idle' | 'done' | 'error';

export interface Activity {
  kind: ActivityKind;
  text: string;
  /** ms (Date.now()). */
  at: number;
  /** From the terminal heuristic, not an exact signal. */
  approximate: boolean;
}

const [activities, setActivities] = createStore<Record<string, Activity>>({});

/** Reactive; null before anything happened. */
export function activityOf(panelId: string): Activity | null {
  return activities[panelId] ?? null;
}

export function setActivity(
  panelId: string,
  kind: ActivityKind,
  text: string,
  approximate = false,
): void {
  const current = activities[panelId];
  if (current?.kind === kind && current.text === text && current.approximate === approximate) {
    return;
  }
  setActivities(panelId, { kind, text, at: Date.now(), approximate });
}

export function clearActivity(panelId: string): void {
  setActivities(
    produce((all) => {
      delete all[panelId];
    }),
  );
}

const ACTION_KEYS: Record<ToolAction, I18nKey> = {
  run: 'doing.run',
  edit: 'doing.edit',
  read: 'doing.read',
  search: 'doing.search',
  web: 'doing.web',
  delegate: 'doing.delegate',
  plan: 'doing.plan',
  tool: 'doing.tool',
};

/** "Running npm test", "Editing src/app.ts", "Using mcp__x" (no target: the bare verb). */
export function toolText(action: ToolAction | null, target: string | null): string {
  const key = ACTION_KEYS[action ?? 'tool'];
  return target ? t(key, { target }) : t(`${key}.bare` as I18nKey);
}

/** The bubble text of a kind without a tool. */
export function kindText(kind: Exclude<ActivityKind, 'tool'>): string {
  switch (kind) {
    case 'think':
      return t('doing.think');
    case 'wait':
      return t('doing.wait');
    case 'idle':
      return '';
    case 'done':
      return t('doing.done');
    case 'error':
      return t('doing.error');
  }
}
