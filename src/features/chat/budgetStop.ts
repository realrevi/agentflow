// The hard budget stop in a chat: a send refused with kind `budget` is kept (text and
// files) until the user picks "Allow once" (a single-use token from usage_allow_once, then
// the same send again) or cancels (the text goes back into the prompt bar).
import { createSignal } from 'solid-js';
import { toAppError } from '../../shared/errors';
import type { AllowOnceGrant, Attachment } from '../../shared/types';

export interface BlockedSend {
  /** `null` = a retry of the last prompt. */
  text: string | null;
  attachments: Attachment[];
  providerId: string;
  /** The backend's reason ("monthly budget for … used up ($5.10 of $5.00)"). */
  message: string;
}

export interface BudgetStopDeps {
  allowOnce: (providerId: string) => Promise<AllowOnceGrant>;
  /** Sends again with the token; resolves whether it went out. */
  resend: (text: string | null, attachments: Attachment[], token: string) => Promise<boolean>;
}

export function isBudgetError(err: unknown): boolean {
  return toAppError(err).kind === 'budget';
}

export function createBudgetStop(deps: BudgetStopDeps) {
  const [blocked, setBlocked] = createSignal<BlockedSend | null>(null);
  const [allowing, setAllowing] = createSignal(false);

  /** Lets the blocked send through once. Resolves whether it was sent. */
  async function allow(): Promise<boolean> {
    const send = blocked();
    if (!send || allowing()) return false;
    setAllowing(true);
    try {
      const grant = await deps.allowOnce(send.providerId);
      setBlocked(null);
      return await deps.resend(send.text, send.attachments, grant.token);
    } finally {
      setAllowing(false);
    }
  }

  /** Gives up on the blocked send; returns it so its text can go back to the bar. */
  function dismiss(): BlockedSend | null {
    const send = blocked();
    setBlocked(null);
    return send;
  }

  return { blocked, allowing, block: setBlocked, allow, dismiss };
}

export type BudgetStop = ReturnType<typeof createBudgetStop>;
