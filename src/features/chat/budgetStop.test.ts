import { describe, expect, it, vi } from 'vitest';
import type { AllowOnceGrant, Attachment } from '../../shared/types';
import { createBudgetStop, isBudgetError } from './budgetStop';

const file: Attachment = { path: 'src/a.rs', content: 'fn a() {}' };

function setup(resendOk = true) {
  const allowOnce = vi.fn(async (providerId: string): Promise<AllowOnceGrant> => ({
    token: 'tok-1',
    providerId,
    expiresAt: 1,
  }));
  const resend = vi.fn(async () => resendOk);
  const stop = createBudgetStop({ allowOnce, resend });
  return { stop, allowOnce, resend };
}

describe('hard budget stop in a chat', () => {
  it('recognizes the backend error kind', () => {
    expect(isBudgetError({ kind: 'budget', message: 'monthly budget for p1 used up' })).toBe(true);
    expect(isBudgetError({ kind: 'provider', message: 'nope' })).toBe(false);
    expect(isBudgetError(new Error('x'))).toBe(false);
  });

  it('keeps the blocked send, then allow once resends it with the token', async () => {
    const { stop, allowOnce, resend } = setup();
    stop.block({ text: 'hello', attachments: [file], providerId: 'p1', message: 'used up' });
    expect(stop.blocked()?.text).toBe('hello');

    await expect(stop.allow()).resolves.toBe(true);
    expect(allowOnce).toHaveBeenCalledWith('p1');
    expect(resend).toHaveBeenCalledWith('hello', [file], 'tok-1');
    expect(stop.blocked()).toBeNull();
    expect(stop.allowing()).toBe(false);
    // Nothing left to allow: a second click does nothing.
    await expect(stop.allow()).resolves.toBe(false);
    expect(allowOnce).toHaveBeenCalledTimes(1);
  });

  it('cancel hands the prompt back without sending', async () => {
    const { stop, allowOnce, resend } = setup();
    stop.block({ text: null, attachments: [], providerId: 'p1', message: 'used up' });
    expect(stop.dismiss()).toEqual({
      text: null,
      attachments: [],
      providerId: 'p1',
      message: 'used up',
    });
    expect(stop.blocked()).toBeNull();
    expect(allowOnce).not.toHaveBeenCalled();
    expect(resend).not.toHaveBeenCalled();
  });

  it('reports a resend that failed', async () => {
    const { stop } = setup(false);
    stop.block({ text: 'hi', attachments: [], providerId: 'p2', message: 'used up' });
    await expect(stop.allow()).resolves.toBe(false);
  });
});
