// A chat panel's phase 6 wiring: the session usage for the corner badge, the live
// estimate (and whether it reached the compact threshold), auto-compact once per crossing,
// budget alerts flashing the character, and which attachments the chat sent before.
import { createEffect, on, onCleanup } from 'solid-js';
import type { Attachment, Estimate } from '../../shared/types';
import { flashAgentState } from '../agentui';
import { budgetCovers, createPanelUsage, onBudgetAlert } from '../tokens';
import { createAutoCompact } from './autoCompact';
import { createEstimate } from './estimate';
import type { ChatSession } from './session';

export interface ChatUsageOptions {
  panelId: string;
  workspace: string | null;
  conversationId: string;
  session: ChatSession;
  draft: () => string;
  files: () => readonly Attachment[];
  ready: () => boolean;
}

export function createChatUsage(options: ChatUsageOptions) {
  const { session } = options;
  const usage = createPanelUsage(options.panelId);
  const estimate = createEstimate({
    workspace: options.workspace,
    conversationId: options.conversationId,
    draft: options.draft,
    files: options.files,
    revision: () => [session.messages().length, session.conversation()?.compaction?.through],
    ready: () => options.ready() && !session.streaming(),
  });
  const nearLimit = () => {
    const current: Estimate | null = estimate();
    return current !== null && current.share >= current.threshold;
  };

  const autoCompact = createAutoCompact();
  createEffect(
    on(estimate, (current) => {
      if (autoCompact.observe(current)) void session.compact();
    }),
  );

  // A budget alert for this chat's provider (or all sources) flashes the character.
  onCleanup(
    onBudgetAlert((status) => {
      if (budgetCovers(status.scope, session.conversation()?.providerId)) {
        flashAgentState(options.panelId, 'error');
      }
    }),
  );

  const sentBefore = (path: string) =>
    session.messages().some((message) => message.attachments.some((file) => file.path === path));

  return { usage, estimate, nearLimit, sentBefore };
}
