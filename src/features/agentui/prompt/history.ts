// One panel's prompt history: loaded once from the backend, appended locally on send.
// Stored per workspace by Rust (src-tauri/src/prompts), capped at 200 per panel.
import { createSignal, getOwner, onCleanup } from 'solid-js';
import { ipc } from '../../../shared/ipc';
import type { PromptEntry } from '../../../shared/types';
import { reportError } from '../../i18n';

export interface PromptHistory {
  /** Oldest first. */
  entries: () => readonly PromptEntry[];
  add(text: string): void;
}

export const MAX_PROMPTS = 200;

export function createPromptHistory(workspace: string | null, key: string): PromptHistory {
  const [entries, setEntries] = createSignal<readonly PromptEntry[]>([]);
  let disposed = false;
  if (getOwner()) onCleanup(() => (disposed = true));
  ipc.promptsList(workspace, key).then(
    (loaded) => !disposed && setEntries((local) => merge(loaded, local)),
    (err: unknown) => reportError('error.promptHistory', err),
  );
  return {
    entries,
    add(text) {
      setEntries((list) => merge(list, [{ text, at: Date.now() }]));
      ipc.promptsAdd(workspace, key, text).catch((err: unknown) => {
        reportError('error.promptHistory', err);
      });
    },
  };
}

/** Appends `extra`, collapsing a repeat of the last entry (like the backend does). */
export function merge(
  base: readonly PromptEntry[],
  extra: readonly PromptEntry[],
): readonly PromptEntry[] {
  const out = [...base];
  for (const entry of extra) {
    if (out[out.length - 1]?.text === entry.text) out.pop();
    out.push(entry);
  }
  return out.slice(-MAX_PROMPTS);
}
