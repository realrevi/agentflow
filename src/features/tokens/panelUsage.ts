// One panel's session usage for its corner badge: loaded on mount, reloaded when the
// tracker reports new usage for this panel (or new prices). Never polled.
import { createSignal, onCleanup } from 'solid-js';
import { ipc } from '../../shared/ipc';
import type { Totals } from '../../shared/types';
import { badgeText } from './format';
import { onPanelUsage } from './usageStore';

/** Badge text ("12.4k · $0.08"), or `null` while the panel has no usage yet. */
export function createPanelUsage(panelId: string, since = Date.now()): () => string | null {
  const [totals, setTotals] = createSignal<Totals | null>(null);
  let disposed = false;
  const load = () => {
    ipc
      .usagePanelTotals(panelId, since)
      .then((next) => {
        if (!disposed) setTotals(next);
      })
      .catch((err: unknown) => console.debug('[agentflow] panel usage:', err));
  };
  load();
  const stop = onPanelUsage(panelId, load);
  onCleanup(() => {
    disposed = true;
    stop();
  });
  return () => {
    const current = totals();
    return current && current.requests > 0 ? badgeText(current) : null;
  };
}
