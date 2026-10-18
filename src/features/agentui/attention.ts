// Panels that need the user: a turn ended, a permission is pending, a workstream manager
// asked a question, or an error. Their frame, tab, sidebar entry and Stage character glow
// in `--af-attention` (the agent bold gold unless set). A pending permission glows until it
// is answered; the rest clears when the user focuses the panel and interacts with it (a
// click, typing, or scrolling it while focused), never on hover. Optionally an OS
// notification while the window is in the background (no sound).
import { createEffect, createRoot } from 'solid-js';
import { createStore, produce } from 'solid-js/store';
import { ipc } from '../../shared/ipc';
import { t, type I18nKey } from '../i18n';
import { settings } from '../settings/settingsStore';
import { agentState } from './state';

export type AttentionReason = 'turn' | 'permission' | 'question' | 'error';

const [reasons, setReasons] = createStore<Record<string, AttentionReason>>({});

/** Reactive. */
export function attentionOf(panelId: string): AttentionReason | null {
  if (agentState(panelId) === 'wait') return 'permission';
  return reasons[panelId] ?? null;
}

/** Reactive: attention and the glow is on. */
export function glowing(panelId: string): boolean {
  return settings.workstreams.glow && attentionOf(panelId) !== null;
}

export function requestAttention(panelId: string, reason: AttentionReason, who: string): void {
  setReasons(panelId, reason);
  notifyIfAway(panelId, reason, who);
}

export function clearAttention(panelId: string): void {
  if (reasons[panelId] === undefined) return;
  setReasons(
    produce((all) => {
      delete all[panelId];
    }),
  );
}

/** Whether an interaction with a panel clears its glow: a click or typing always focuses it;
 * a wheel over a panel that is not focused is just hovering. */
export function interactionClears(type: string, panelActive: boolean): boolean {
  if (type === 'pointerdown' || type === 'keydown') return true;
  return type === 'wheel' && panelActive;
}

const NOTIFY_KEYS: Record<AttentionReason, I18nKey> = {
  turn: 'attention.notify.turn',
  permission: 'attention.notify.permission',
  question: 'attention.notify.question',
  error: 'attention.notify.error',
};
const NOTIFY_GAP_MS = 10_000;
const notified = new Map<string, number>();

/** OS notification: opt-in, only while the window is in the background, at most one per
 * panel every few seconds. */
export function shouldNotify(
  enabled: boolean,
  windowFocused: boolean,
  lastAt: number,
  now: number,
) {
  return enabled && !windowFocused && now - lastAt >= NOTIFY_GAP_MS;
}

function notifyIfAway(panelId: string, reason: AttentionReason, who: string): void {
  const now = Date.now();
  const focused = typeof document !== 'undefined' && document.hasFocus();
  if (!shouldNotify(settings.workstreams.notify, focused, notified.get(panelId) ?? 0, now)) return;
  notified.set(panelId, now);
  void ipc.notify(t('attention.notify.title'), t(NOTIFY_KEYS[reason], { name: who }));
}

/** Keeps `--af-attention` on the document in step with the settings (app start). */
export function installAttentionStyle(): void {
  createRoot(() => {
    createEffect(() => {
      const color = settings.workstreams.glowColor ?? settings.appearance.agentBold.color;
      document.documentElement.style.setProperty('--af-attention', color);
    });
  });
}
