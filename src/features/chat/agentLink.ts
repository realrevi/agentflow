// Phase 9 wiring of one API chat panel: its delivery port (a handoff is sent like a typed
// prompt), the turn end with the final assistant message as the report, the bubble activity
// from the agent stream (status and running tool), the glow on errors and the extra system
// text a workstream gives its manager. Call inside the panel's component.
import { createEffect, onCleanup } from 'solid-js';
import type { ChatMessage } from '../../shared/types';
import {
  agentState,
  clearActivity,
  extraSystemOf,
  kindText,
  registerPort,
  reportTurnEnded,
  requestAttention,
  setActivity,
} from '../agentui';
import { t } from '../i18n';
import type { ChatSession, Ending } from './session';

export interface ChatLinkOptions {
  panelId: string;
  ready: () => boolean;
  deliver: (text: string) => boolean;
  label: () => string;
}

/** The text of the turn's last assistant message: what the agent finally said. */
export function finalText(messages: readonly ChatMessage[]): string {
  const last = [...messages].reverse().find((message) => message.role === 'assistant');
  if (!last) return '';
  if (last.content.trim()) return last.content.trim();
  const parts = (last.parts ?? []).flatMap((part) => (part.type === 'text' ? [part.text] : []));
  return parts.join('').trim();
}

/** Session options the link provides (pass into createChatSession). */
export function chatLinkHooks(
  panelId: string,
  label: () => string,
  messages: () => readonly ChatMessage[],
) {
  return {
    extraSystem: () => extraSystemOf(panelId),
    onEnded: (end: Ending) => {
      if (end.type === 'error') {
        requestAttention(panelId, 'error', label());
        return;
      }
      const interrupted = end.reason === 'cancelled';
      reportTurnEnded(panelId, {
        report: interrupted ? '' : finalText(messages()),
        source: 'stop',
        interrupted,
      });
    },
  };
}

export function linkChat(options: ChatLinkOptions, session: ChatSession): void {
  const { panelId } = options;
  onCleanup(
    registerPort(panelId, {
      kind: 'api',
      ready: () => options.ready() && !session.streaming() && !session.compacting(),
      deliver: options.deliver,
      label: options.label,
    }),
  );
  createEffect(() => {
    const state = agentState(panelId);
    const tool = session.agent.live.status();
    if (state === 'wait') setActivity(panelId, 'wait', kindText('wait'));
    else if ((state === 'think' || state === 'work') && tool) setActivity(panelId, 'tool', tool);
    else if (state === 'think') setActivity(panelId, 'think', kindText('think'));
    else if (state === 'work') setActivity(panelId, 'think', t('doing.work'));
    else if (state === 'done') setActivity(panelId, 'done', kindText('done'));
    else if (state === 'error') {
      const message = session.error()?.message;
      setActivity(
        panelId,
        'error',
        message ? t('doing.errorText', { message }) : kindText('error'),
      );
    } else setActivity(panelId, 'idle', '');
  });
  onCleanup(() => clearActivity(panelId));
}
