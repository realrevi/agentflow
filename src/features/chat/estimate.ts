// The live pre-send estimate under the prompt bar: what sending the draft now would cost
// (history + attachments + draft), reloaded shortly after typing stops, when files or the
// history change, and when prices change. Never polled.
import { createEffect, createSignal, on, onCleanup } from 'solid-js';
import { debounce } from '../../shared/debounce';
import { ipc } from '../../shared/ipc';
import type { Attachment, Estimate } from '../../shared/types';
import { onRepriced } from '../tokens';

const DELAY_MS = 350;

export interface EstimateInputs {
  workspace: string | null;
  conversationId: string;
  draft: () => string;
  files: () => readonly Attachment[];
  /** Changes whenever the stored history does (message count, compaction). */
  revision: () => unknown;
  /** Only estimate while the chat has a provider and model. */
  ready: () => boolean;
}

export function createEstimate(inputs: EstimateInputs): () => Estimate | null {
  const [estimate, setEstimate] = createSignal<Estimate | null>(null);
  let ticket = 0;
  let disposed = false;
  const load = async () => {
    if (disposed) return;
    if (!inputs.ready()) {
      setEstimate(null);
      return;
    }
    const mine = ++ticket;
    try {
      const next = await ipc.usageEstimate({
        workspace: inputs.workspace,
        conversationId: inputs.conversationId,
        text: inputs.draft(),
        attachments: [...inputs.files()],
      });
      if (mine === ticket && !disposed) setEstimate(next);
    } catch (err) {
      // An estimate is a hint: never a toast while typing.
      console.debug('[agentflow] estimate:', err);
    }
  };
  const later = debounce(() => void load(), DELAY_MS);
  createEffect(
    on([inputs.draft, inputs.files, inputs.revision, inputs.ready], () => later(), {
      defer: false,
    }),
  );
  onCleanup(onRepriced(later));
  onCleanup(() => {
    disposed = true;
  });
  return estimate;
}
