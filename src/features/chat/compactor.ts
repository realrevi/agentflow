// Compact for one chat: summarizes the older turns (the last `chat.compactKeepTurns` stay
// verbatim); the character thinks meanwhile.
import { createSignal } from 'solid-js';
import { ipc } from '../../shared/ipc';
import type { Compaction } from '../../shared/types';
import type { AgentState } from '../characters';
import { reportError } from '../i18n';

export interface CompactorOptions {
  panelId: string;
  workspace: string | null;
  conversationId: string;
  /** A reply streams or the panel closed. */
  busy: () => boolean;
  state: (state: AgentState) => void;
  onCompacted: (compaction: Compaction | null) => void;
}

export function createCompactor(options: CompactorOptions) {
  const [compacting, setCompacting] = createSignal(false);

  async function compact(): Promise<boolean> {
    if (compacting() || options.busy()) return false;
    setCompacting(true);
    options.state('think');
    try {
      const result = await ipc.chatCompact({
        workspace: options.workspace,
        conversationId: options.conversationId,
        panel: options.panelId,
      });
      options.onCompacted(result.compaction);
      options.state('done');
      return true;
    } catch (err) {
      options.state('error');
      reportError('error.compact', err);
      return false;
    } finally {
      setCompacting(false);
    }
  }

  return { compact, compacting };
}
